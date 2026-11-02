use crate::palette::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Edit,
    Selection,
    View,
    Go,
    Run,
    Terminal,
    Help,
}

impl MenuKind {
    pub const ALL: [MenuKind; 8] = [
        MenuKind::File,
        MenuKind::Edit,
        MenuKind::Selection,
        MenuKind::View,
        MenuKind::Go,
        MenuKind::Run,
        MenuKind::Terminal,
        MenuKind::Help,
    ];

    pub fn title(self) -> &'static str {
        match self {
            MenuKind::File => "File",
            MenuKind::Edit => "Edit",
            MenuKind::Selection => "Selection",
            MenuKind::View => "View",
            MenuKind::Go => "Go",
            MenuKind::Run => "Run",
            MenuKind::Terminal => "Terminal",
            MenuKind::Help => "Help",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Command(Command),
    Separator,
}

use Command as C;
use MenuItem::{Command as Cmd, Separator as Sep};

pub fn items(kind: MenuKind) -> Vec<MenuItem> {
    match kind {
        MenuKind::File => vec![
            Cmd(C::NewFile),
            Cmd(C::NewFolder),
            Sep,
            Cmd(C::OpenFile),
            Cmd(C::OpenFolder),
            Cmd(C::QuickOpen),
            Sep,
            Cmd(C::Save),
            Cmd(C::SaveAll),
            Sep,
            Cmd(C::CloseTab),
            Cmd(C::CloseAllTabs),
            Sep,
            Cmd(C::OpenSettings),
            Sep,
            Cmd(C::Exit),
        ],
        MenuKind::Edit => vec![
            Cmd(C::Undo),
            Cmd(C::Redo),
            Sep,
            Cmd(C::Find),
            Cmd(C::Replace),
            Cmd(C::FindNext),
            Cmd(C::FindPrev),
            Sep,
            Cmd(C::FindInFiles),
            Sep,
            Cmd(C::ToggleComment),
            Cmd(C::DeleteLine),
        ],
        MenuKind::Selection => vec![
            Cmd(C::SelectAll),
            Sep,
            Cmd(C::MoveLineUp),
            Cmd(C::MoveLineDown),
            Cmd(C::DuplicateLineUp),
            Cmd(C::DuplicateLineDown),
            Sep,
            Cmd(C::IndentLines),
            Cmd(C::OutdentLines),
        ],
        MenuKind::View => vec![
            Cmd(C::CommandPalette),
            Sep,
            Cmd(C::ShowExplorer),
            Cmd(C::ShowSearch),
            Cmd(C::ShowSourceControl),
            Cmd(C::ShowRun),
            Cmd(C::ShowExtensions),
            Sep,
            Cmd(C::ShowProblems),
            Cmd(C::ShowOutput),
            Cmd(C::ToggleTerminal),
            Sep,
            Cmd(C::ToggleSidebar),
            Cmd(C::TogglePanel),
            Cmd(C::OpenExtensionPanel),
            Cmd(C::ToggleActivityBar),
            Cmd(C::ToggleStatusBar),
            Cmd(C::MaximizePanel),
            Sep,
            Cmd(C::SidebarLeft),
            Cmd(C::SidebarRight),
            Cmd(C::PanelBottom),
            Cmd(C::PanelTop),
            Cmd(C::PanelLeft),
            Cmd(C::PanelRight),
            Sep,
            Cmd(C::ResetLayout),
        ],
        MenuKind::Go => vec![
            Cmd(C::QuickOpen),
            Cmd(C::GoToLine),
            Sep,
            Cmd(C::NextTab),
            Cmd(C::PrevTab),
        ],
        MenuKind::Run => vec![
            Cmd(C::RunTask),
            Sep,
            Cmd(C::SwitchBranch),
            Cmd(C::GitRefresh),
        ],
        MenuKind::Terminal => vec![
            Cmd(C::NewTerminal),
            Cmd(C::ToggleTerminal),
            Cmd(C::KillTerminal),
        ],
        MenuKind::Help => vec![Cmd(C::CommandPalette), Sep, Cmd(C::About)],
    }
}

pub fn context_items(is_dir: bool) -> Vec<MenuItem> {
    let mut v = Vec::new();
    if is_dir {
        v.extend([Cmd(C::NewFile), Cmd(C::NewFolder), Sep]);
    }
    v.extend([
        Cmd(C::RenameItem),
        Cmd(C::DeleteItem),
        Sep,
        Cmd(C::CopyPath),
        Cmd(C::RevealInOs),
    ]);
    if is_dir {
        v.extend([Sep, Cmd(C::RefreshExplorer), Cmd(C::CollapseAll)]);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_menus() -> Vec<Vec<MenuItem>> {
        let mut v: Vec<_> = MenuKind::ALL.iter().map(|k| items(*k)).collect();
        v.push(context_items(true));
        v.push(context_items(false));
        v
    }

    #[test]
    fn every_menu_entry_is_a_listed_command() {
        for menu in all_menus() {
            for it in menu {
                if let Cmd(c) = it {
                    assert!(
                        Command::ALL.contains(&c),
                        "{c:?} is not in the palette list"
                    );
                }
            }
        }
    }

    #[test]
    fn menus_have_no_leading_trailing_or_doubled_separators_and_no_repeats() {
        for menu in all_menus() {
            assert!(!menu.is_empty());
            assert_ne!(menu.first(), Some(&Sep));
            assert_ne!(menu.last(), Some(&Sep));
            assert!(menu.windows(2).all(|w| w != [Sep, Sep]));
            let cmds: Vec<_> = menu
                .iter()
                .filter_map(|i| if let Cmd(c) = i { Some(*c) } else { None })
                .collect();
            let mut dedup = cmds.clone();
            dedup.dedup();
            assert_eq!(cmds.len(), dedup.len());
        }
    }

    #[test]
    fn menu_labels_drop_the_category() {
        assert_eq!(Command::Save.menu_label(), "Save");
        assert_eq!(Command::GoToLine.menu_label(), "Go to Line...");
        assert_eq!(Command::About.menu_label(), "About nanti.");
    }
}
