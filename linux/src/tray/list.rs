use mini_sansio_dbus::messages::sni_client::dbusmenu::{DBusMenuItem, DBusMenuList};
use std::collections::VecDeque;

pub struct MenuList {
    strings: VecDeque<String>,
    menu: Vec<DBusMenuItem<Self, String>>,
}

impl MenuList {
    pub const QUIT_ID: i32 = 1;
    pub const SEPARATOR_ID: i32 = 2;

    pub(crate) fn new() -> Self {
        let strings = VecDeque::new();
        let menu = Self::menu(&strings);
        Self { strings, menu }
    }

    pub(crate) fn push(&mut self, text: String) {
        self.strings.push_front(text);
        if self.strings.len() > 5 {
            self.strings.pop_back();
        }
        self.menu = Self::menu(&self.strings);
    }

    fn menu(strings: &VecDeque<String>) -> Vec<DBusMenuItem<Self, String>> {
        let mut menu = vec![];

        for (string, id) in strings.iter().zip(10..) {
            menu.push(DBusMenuItem::Regular {
                id,
                label: string.clone(),
                enabled: false,
                visible: true,
            });
        }

        menu.push(DBusMenuItem::Separator {
            id: Self::SEPARATOR_ID,
            visible: true,
        });
        menu.push(DBusMenuItem::Regular {
            id: Self::QUIT_ID,
            label: "Quit".to_string(),
            enabled: true,
            visible: true,
        });

        menu
    }
}

impl DBusMenuList<String> for MenuList {
    fn iter(&self) -> impl Iterator<Item = &DBusMenuItem<Self, String>> {
        self.menu.iter()
    }
}
