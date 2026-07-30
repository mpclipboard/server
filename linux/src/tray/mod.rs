use anyhow::{Context, Result};

mod registration;
use mini_sansio_dbus::{
    IncomingMessage, OutgoingQueue,
    messages::{
        EmptyMethodReturn,
        org_freedesktop_dbus::{NameHasOwner, NameOwnerChangedSubscribe, RequestName},
        sni_client::{
            dbusmenu::StatusNotifierMenuHandler,
            sni::{StatusNotifierActivateEvent, StatusNotifierItemHandler, StatusNotifierWatcher},
        },
    },
    messaging::reply_handler::ReplyHandler,
};
use mpclipboard_generic_client::Connectivity;
pub use registration::Registration;

mod list;
pub use list::MenuList;

mod menu;
pub use menu::TrayMenu;

use crate::dbus::DBusQueue;

pub struct Tray {
    sni_has_owner_reply: ReplyHandler<NameHasOwner>,
    registration: Registration,
    sni: StatusNotifierItemHandler<'static>,
    menu: TrayMenu,
}

impl Tray {
    pub(crate) fn new(queue: &mut DBusQueue) -> Result<Self> {
        queue.push_without_reply::<RequestName>(Registration::DBUS_NAME)?;
        queue.push_without_reply::<NameOwnerChangedSubscribe>(())?;
        let sni_has_owner_reply =
            queue.push_with_reply(NameHasOwner, "org.kde.StatusNotifierWatcher")?;
        let registration = Registration::Unset;
        let sni = StatusNotifierItemHandler::new(Registration::DBUS_NAME);
        let menu = TrayMenu::new();

        Ok(Self {
            sni_has_owner_reply,
            registration,
            sni,
            menu,
        })
    }

    pub(crate) const fn received_exit(&self) -> bool {
        self.menu.received_exit()
    }

    pub(crate) fn set_connectivity(
        &mut self,
        connectivity: Connectivity,
        queue: &mut DBusQueue,
    ) -> Result<()> {
        self.menu.set_connectivity(connectivity, queue)
    }

    pub(crate) fn push(&mut self, text: String, queue: &mut DBusQueue) -> Result<()> {
        self.menu.push(text, queue)
    }

    pub(crate) fn handle(
        &mut self,
        message: IncomingMessage<'_>,
        queue: &mut DBusQueue,
    ) -> Result<()> {
        if let Some(has_owner) = self.sni_has_owner_reply.handle(message)? {
            if has_owner {
                self.registration.host_appeared(queue)?;
            } else {
                self.registration.host_disappeared();
            }
        }

        if let Some(event) = StatusNotifierWatcher::handle(message)? {
            match event {
                StatusNotifierWatcher::Appeared { .. } => {
                    self.registration.host_appeared(queue)?;
                }
                StatusNotifierWatcher::Disappeared { .. } => {
                    self.registration.host_disappeared();
                }
            }
        }

        self.registration.handle_message(message);

        let mut reply = [0; 8 * 1_024];
        if let Some(reply) = StatusNotifierMenuHandler::handle(
            &mut reply,
            message,
            Registration::DBUS_NAME,
            TrayMenu::PATH,
            &mut self.menu,
        )? {
            let _ = queue.push_raw(reply);
        } else if let Some(reply) = self.sni.handle(&mut reply, message, &self.menu)? {
            let _ = queue.push_raw(reply);
        } else if StatusNotifierActivateEvent::handle(message) {
            println!("Activate called");
            let sender = message.sender.context("no Sender")?;
            queue.push_without_reply::<EmptyMethodReturn>((sender, message.serial))?;
        }
        Ok(())
    }
}
