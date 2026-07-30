use crate::{dbus::DBusQueue, tray::MenuList};
use anyhow::Result;
use mini_sansio_dbus::messages::sni_client::{
    dbusmenu::{DBusMenuData, DBusMenuEvent, DBusMenuEventKind, LayoutUpdatedSignal},
    sni::{
        IconPixmap, NewIconSignal, StatusNotifierItemCategory, StatusNotifierItemData,
        StatusNotifierItemStatus,
    },
};
use mpclipboard_generic_client::Connectivity;

pub struct TrayMenu {
    revision: u32,
    icon: &'static [u8],
    menu: MenuList,
    received_exit: bool,
}

impl TrayMenu {
    pub const PATH: &str = "/Menu";

    const GREEN: &[u8] = include_bytes!("../../assets/green.argb32");
    const RED: &[u8] = include_bytes!("../../assets/red.argb32");
    const YELLOW: &[u8] = include_bytes!("../../assets/yellow.argb32");

    pub(crate) fn new() -> Self {
        Self {
            revision: 0,
            icon: Self::RED,
            menu: MenuList::new(),
            received_exit: false,
        }
    }

    pub(crate) fn set_connectivity(
        &mut self,
        connectivity: Connectivity,
        queue: &mut DBusQueue,
    ) -> Result<()> {
        self.icon = match connectivity {
            Connectivity::Connecting => Self::YELLOW,
            Connectivity::Connected => Self::GREEN,
            Connectivity::Disconnected => Self::RED,
        };
        queue.push_without_reply::<NewIconSignal>(())?;
        Ok(())
    }

    pub(crate) fn push(&mut self, text: String, queue: &mut DBusQueue) -> Result<()> {
        self.menu.push(text);
        self.revision = self.revision.saturating_add(1);
        queue.push_without_reply::<LayoutUpdatedSignal>(("/Menu", self.revision(), 0))?;
        Ok(())
    }

    pub(crate) const fn received_exit(&self) -> bool {
        self.received_exit
    }
}

impl StatusNotifierItemData for TrayMenu {
    fn id(&self) -> &'static str {
        "mpclipboard-client"
    }

    fn title(&self) -> &'static str {
        "MPClipboard client"
    }

    fn status(&self) -> StatusNotifierItemStatus {
        StatusNotifierItemStatus::Active
    }

    fn category(&self) -> StatusNotifierItemCategory {
        StatusNotifierItemCategory::ApplicationStatus
    }

    fn icon_name(&self) -> &'static str {
        ""
    }

    fn icon_pixmap(&self) -> Option<IconPixmap<'_>> {
        Some(IconPixmap {
            width: 32,
            height: 32,
            argb: self.icon,
        })
    }

    fn menu(&self) -> &'static str {
        Self::PATH
    }

    fn item_is_menu(&self) -> bool {
        false
    }
}

impl DBusMenuData<String> for TrayMenu {
    type List = MenuList;

    fn revision(&self) -> u32 {
        self.revision
    }

    fn menu(&self) -> &Self::List {
        &self.menu
    }

    fn event(&mut self, event: DBusMenuEvent<'_>) {
        if matches!(event.kind, DBusMenuEventKind::Clicked) && event.id == MenuList::QUIT_ID {
            log::trace!("Received DBusMenuEvent exit");
            self.received_exit = true;
        }
    }
}
