package org.enso.gateway.protocol.request

import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder
import org.enso.gateway.protocol.TextEdit
import org.enso.gateway.protocol.request.Param.VersionedTextDocumentIdentifier

case class TextDocumentEdit(
  textDocument: VersionedTextDocumentIdentifier,
  edits: Seq[TextEdit]
)

object TextDocumentEdit {
  implicit val TextDocumentEditDecoder: Decoder[TextDocumentEdit] =
    deriveDecoder
}