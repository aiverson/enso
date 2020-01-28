package org.enso.languageserver

import akka.actor.ActorRef

trait ErrorResponse {
  def id: Id

  def msg: String

  def replyTo: ActorRef
}

object ErrorResponse {

  case class InvalidRequest(id: Id, msg: String, replyTo: ActorRef)
      extends ErrorResponse

  case class ServerNotInitialized(id: Id, msg: String, replyTo: ActorRef)
      extends ErrorResponse

}
