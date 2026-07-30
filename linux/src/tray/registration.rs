use crate::dbus::DBusQueue;
use anyhow::Result;
use mini_sansio_dbus::{
    IncomingMessage, MessageType, messages::sni_client::sni::RegisterStatusNotifierItem,
    messaging::reply_handler::ReplyHandler,
};

pub enum Registration {
    Unset,
    Registering(ReplyHandler<RegisterStatusNotifierItem>),
    Registered,
}

impl Registration {
    pub(crate) const DBUS_NAME: &str = "org.mpclipboard.client";

    pub(crate) fn host_appeared(&mut self, queue: &mut DBusQueue) -> Result<()> {
        let Self::Unset = self else { return Ok(()) };
        let handler = queue.push_with_reply(RegisterStatusNotifierItem, Self::DBUS_NAME)?;
        *self = Self::Registering(handler);
        log::trace!(
            "sent RegisterStatusNotifierItem({}): Unset -> Registering",
            Self::DBUS_NAME
        );
        Ok(())
    }

    pub(crate) fn host_disappeared(&mut self) {
        log::trace!("host disappeared: -> Unset");
        *self = Self::Unset;
    }

    pub(crate) fn handle_message(&mut self, message: IncomingMessage<'_>) {
        let Self::Registering(handler) = self else {
            return;
        };

        if matches!(message.message_type, MessageType::MethodReturn)
            && message
                .reply_serial
                .is_some_and(|reply_serial| reply_serial == handler.serial)
        {
            log::trace!("Registering -> Registered");
            *self = Self::Registered;
        }
    }
}
