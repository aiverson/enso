package org.enso.gateway.protocol

import io.circe.{ACursor, Decoder, DecodingFailure}
import org.enso.gateway.Protocol.jsonRpcVersion
import org.enso.gateway.protocol.request.Params
import org.enso.gateway.protocol.request.Params.{
  InitializeParams,
  InitializedParams
}

/**
  * Helper object for decoding [[Notification]]
  */
object NotificationDecoder {

  /**
    * @tparam P
    * @return Circe decoder for notifications and notification fields of requests
    */
  def instance[P <: Params]: Decoder[Notification[P]] =
    cursor => {
      val jsonrpcCursor = cursor.downField(Notification.jsonrpcField)
      val methodCursor  = cursor.downField(Notification.methodField)
      val paramsCursor  = cursor.downField(Notification.paramsField)
      // Field `jsonrpc` must be correct
      val jsonrpcResult = validateJsonrpc(jsonrpcCursor)
      val methodResult  = Decoder[String].tryDecode(methodCursor)
      // Discriminator is field `method`
      val paramsResult = methodResult
        .flatMap(selectParamsDecoder(_).tryDecode(paramsCursor))
      for {
        jsonrpc <- jsonrpcResult
        method  <- methodResult
        params  <- paramsResult
      } yield Notification[P](jsonrpc, method, params)
    }

  private def selectParamsDecoder[P <: Params](
    method: String
  ): Decoder[Option[P]] =
    (method match {
      // All requests
      case Requests.Initialize.method =>
        Decoder[Option[InitializeParams]]

      // All notifications
      case Notifications.Initialized.method =>
        Decoder[Option[InitializedParams]]

      case m =>
        Decoder.failed(
          RequestOrNotificationDecoder.unknownMethodFailure(m)
        )
    }).asInstanceOf[Decoder[Option[P]]]

  private def validateJsonrpc[P <: Params](
    jsonrpcCursor: ACursor
  ): Decoder.Result[String] = {
    Decoder[String].tryDecode(jsonrpcCursor).flatMap {
      case version @ `jsonRpcVersion` => Right(version)
      case version =>
        Left(
          wrongJsonRpcVersionFailure(version, jsonrpcCursor)
        )
    }
  }

  private def wrongJsonRpcVersionFailure(
    version: String,
    jsonrpcCursor: ACursor
  ): DecodingFailure =
    DecodingFailure(
      wrongJsonRpcVersionMessage(version),
      jsonrpcCursor.history
    )

  private def wrongJsonRpcVersionMessage(version: String) =
    s"jsonrpc must be $jsonRpcVersion but found $version"

}
