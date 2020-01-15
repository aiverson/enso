package org.enso.gateway

import akka.NotUsed
import akka.actor.ActorSystem
import akka.event.{Logging, LoggingAdapter}
import akka.http.scaladsl.Http
import akka.http.scaladsl.model.ws.BinaryMessage
import akka.http.scaladsl.model.ws.Message
import akka.http.scaladsl.model.ws.TextMessage
import akka.http.scaladsl.server.Directives._
import akka.http.scaladsl.server.Route
import akka.stream.ActorMaterializer
import akka.stream.scaladsl.Flow
import akka.stream.scaladsl.Sink
import akka.stream.scaladsl.Source
import akka.util.Timeout
import com.typesafe.config.{Config, ConfigFactory}

import scala.concurrent.duration._
import scala.util.Failure
import scala.util.Success

object Server {

  /**
    * Describes endpoint to which [[Server]] can bind (host, port, route) and timeout for waiting response
    */
  object Config {
    val host: String  = serverConfig.getString(hostPath)
    val port: Int     = serverConfig.getInt(portPath)
    val route: String = serverConfig.getString(routePath)
    implicit val timeout: Timeout = Timeout(
      serverConfig.getLong(timeoutPath).seconds
    )
    val addressString: String = s"ws://$host:$port"

    private val gatewayPath = "gateway"
    private val serverPath  = "server"
    private val hostPath    = "host"
    private val portPath    = "port"
    private val routePath   = "route"
    private val timeoutPath = "timeout"
    private val gatewayConfig: Config =
      ConfigFactory.load.getConfig(gatewayPath)
    private val serverConfig: Config = gatewayConfig.getConfig(serverPath)
  }
}

/** WebSocket server supporting synchronous request-response protocol.
  *
  * Server when run binds to endpoint and accepts establishing web socket
  * connection for any number of peers.
  *
  * Server replies to each incoming text request with a single text response, no response for notifications.
  * Server accepts a single Text Message from a peer and responds with another Text Message.
  *
  * @param protocol Encapsulates encoding JSONs
  */
class Server(protocol: Protocol)(
  implicit
  system: ActorSystem,
  materializer: ActorMaterializer
) {

  import system.dispatcher
  import Server.Config.timeout

  val log: LoggingAdapter = Logging.getLogger(system, this)

  /** Akka stream defining server behavior.
    *
    * Incoming [[TextMessage]]s are replied to (see [[getTextOutput]]).
    * Incoming binary messages are ignored.
    */
  val handlerFlow: Flow[Message, TextMessage.Strict, NotUsed] =
    Flow[Message]
      .flatMapConcat {
        case tm: TextMessage =>
          val strict = tm.textStream.fold("")(_ + _)
          strict
            .flatMapConcat(
              input =>
                Source
                  .fromFuture(
                    protocol.getTextOutput(input)
                  )
            )
            .flatMapConcat {
              case Some(input) => Source.single(TextMessage(input))
              case None        => Source.empty
            }
        case bm: BinaryMessage =>
          bm.dataStream.runWith(Sink.ignore)
          Source.empty
      }

  /** Server behavior upon receiving HTTP request.
    *
    * As server implements websocket-based protocol, this implementation accepts
    * only GET requests to set up WebSocket connection.
    *
    * The request's URI is not checked.
    */
  val route: Route =
    path(Server.Config.route) {
      get {
        handleWebSocketMessages(handlerFlow)
      }
    }

  /** Starts a HTTP server listening at the given endpoint.
    *
    * Function is asynchronous, will return immediately. If the server fails to
    * start, function will exit the process with a non-zero code.
    */
  def run(): Unit = {
    val bindingFuture =
      Http().bindAndHandle(
        handler   = route,
        interface = Server.Config.host,
        port      = Server.Config.port
      )

    bindingFuture
      .onComplete {
        case Success(_) =>
          val serverOnlineMessage =
            s"Server online at ${Server.Config.addressString}"
          val shutDownMessage = "Press ENTER to shut down"
          Seq(
            serverOnlineMessage,
            shutDownMessage
          ).foreach(log.info)
        case Failure(exception) =>
          val err = s"Failed to start server: $exception"
          log.error(err)
          system.terminate()
          System.exit(1)
      }
  }
}
