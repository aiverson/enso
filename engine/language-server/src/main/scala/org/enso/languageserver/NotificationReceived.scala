package org.enso.languageserver

/** Language server responses to [[Notifications]]. */
object NotificationReceived {

  /** Language server response to [[Notifications.Initialized]]. */
  case object Initialized

  /** Language server response to [[Notifications.Exit]]. */
  case object Exit

  /** */
  case object DidOpenTextDocument

  /** */
  case object DidChangeTextDocument

  /** */
  case object DidSaveTextDocument

  /** */
  case object DidCloseTextDocument

}
