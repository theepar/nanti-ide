use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ui::model::{Model, SaveOutcome};
use ui::palette::Command as Cmd;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tmp(tag: &str) -> Tmp {
    let d = std::env::temp_dir().join(format!("nanti-fmt-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Tmp(d)
}

fn have(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn open_dirty(m: &mut Model, file: &Path) {
    m.open_file(file);
    let e = m.active_editor_mut().unwrap();
    e.move_doc_end(false);
    e.insert("\n");
}

#[test]
fn saving_formats_the_file_when_format_on_save_is_on() {
    if !have("rustfmt") {
        return;
    }
    let t = tmp("save");
    let file = t.0.join("a.rs");
    std::fs::write(&file, "fn main(){let x=1;}\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.format_on_save = true;
    open_dirty(&mut m, &file);
    assert_eq!(m.save_active(), SaveOutcome::Saved);
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(on_disk, "fn main() {\n    let x = 1;\n}\n");
    assert_eq!(m.active_editor_mut().unwrap().text(), on_disk);
    assert!(!m.active().unwrap().is_dirty());
}

#[test]
fn saving_leaves_the_text_alone_when_format_on_save_is_off() {
    let t = tmp("off");
    let file = t.0.join("a.rs");
    std::fs::write(&file, "fn main(){let x=1;}\n").unwrap();
    let mut m = Model::new(t.0.clone());
    open_dirty(&mut m, &file);
    assert_eq!(m.save_active(), SaveOutcome::Saved);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn main(){let x=1;}\n\n"
    );
}

#[test]
fn a_formatter_that_fails_never_blocks_the_save() {
    if !have("rustfmt") {
        return;
    }
    let t = tmp("fail");
    let file = t.0.join("broken.rs");
    std::fs::write(&file, "fn main( {\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.format_on_save = true;
    open_dirty(&mut m, &file);
    assert_eq!(m.save_active(), SaveOutcome::Saved);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn main( {\n\n",
        "the unformatted text is saved as typed"
    );
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.starts_with("rustfmt")),
        "{:?}",
        m.message
    );
}

#[test]
fn format_document_works_without_saving_and_undoes_in_one_step() {
    if !have("gofmt") {
        return;
    }
    let t = tmp("doc");
    let file = t.0.join("a.go");
    std::fs::write(&file, "package main\nfunc main(){\nx:=1\n_=x}\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.open_file(&file);
    m.run_command(Cmd::FormatDocument);
    let formatted = "package main\n\nfunc main() {\n\tx := 1\n\t_ = x\n}\n";
    assert_eq!(m.active_editor_mut().unwrap().text(), formatted);
    assert!(m.active().unwrap().is_dirty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "package main\nfunc main(){\nx:=1\n_=x}\n",
        "nothing is written until the file is saved"
    );
    m.run_command(Cmd::FormatDocument);
    assert_eq!(m.message.as_deref(), Some("Already formatted"));
    assert!(m.active_editor_mut().unwrap().undo());
    assert_eq!(
        m.active_editor_mut().unwrap().text(),
        "package main\nfunc main(){\nx:=1\n_=x}\n"
    );
}

#[test]
fn a_file_type_without_a_formatter_says_so() {
    let t = tmp("none");
    let file = t.0.join("notes.txt");
    std::fs::write(&file, "a  b\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.open_file(&file);
    m.run_command(Cmd::FormatDocument);
    assert_eq!(
        m.message.as_deref(),
        Some("There is no formatter for this file type")
    );
}

#[test]
fn the_toggle_is_remembered_in_the_config_folder() {
    let t = tmp("toggle");
    let mut m = Model::new(t.0.clone());
    m.config_dir = Some(t.0.clone());
    m.load_format_settings();
    assert!(m.format_on_save, "on by default");
    m.run_command(Cmd::ToggleFormatOnSave);
    assert!(!m.format_on_save);
    let mut again = Model::new(t.0.clone());
    again.config_dir = Some(t.0.clone());
    again.load_format_settings();
    assert!(!again.format_on_save);
}

#[test]
fn rename_symbol_replaces_whole_words_only() {
    let t = tmp("rename");
    let file = t.0.join("a.rs");
    std::fs::write(&file, "let count = 1; let counter = count + 1;\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.open_file(&file);
    m.active_editor_mut().unwrap().set_cursor(5, false);
    m.run_command(Cmd::RenameSymbol);
    assert_eq!(m.palette.as_ref().unwrap().field.text(), "count");
    let field = &mut m.palette.as_mut().unwrap().field;
    field.clear();
    field.insert("total");
    m.accept_palette();
    assert_eq!(
        m.active_editor_mut().unwrap().text(),
        "let total = 1; let counter = total + 1;\n"
    );
}

// Runs the real Prettier that ships inside the Prettier extension; needs NANTI_TEST_PRETTIER_EXT to point at the extracted `extension` folder.
#[test]
fn real_prettier_from_the_installed_extension_formats_on_save_and_on_demand() {
    let Some(src) = std::env::var_os("NANTI_TEST_PRETTIER_EXT").map(PathBuf::from) else {
        return;
    };
    let t = tmp("prettier");
    let cfg = t.0.join("cfg");
    let ext = cfg.join("extensions").join("esbenp.prettier-vscode-0.0.0");
    copy_dir(&src, &ext);
    let work = t.0.join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join(".prettierrc"), "{ \"singleQuote\": true }\n").unwrap();

    let mut m = Model::new(work.clone());
    m.config_dir = Some(cfg.clone());
    m.refresh_extensions();
    assert!(
        m.extensions.iter().any(|x| x.dir == ext),
        "the extension is installed: {:?}",
        m.extensions.iter().map(|x| &x.id).collect::<Vec<_>>()
    );

    let cases = [
        (
            "a.ts",
            "const a = \"x\" ;function f( b:number ){return b+1}\n",
            "const a = 'x';\nfunction f(b: number) {\n  return b + 1;\n}\n",
        ),
        (
            "b.json",
            "{\"a\":1,\"b\":[1,2]}\n",
            "{ \"a\": 1, \"b\": [1, 2] }\n",
        ),
        (
            "c.css",
            "a{color:red;margin:0}\n",
            "a {\n  color: red;\n  margin: 0;\n}\n",
        ),
        ("d.md", "#  Title\n\n*  item\n", "# Title\n\n- item\n"),
    ];
    for (name, messy, clean) in cases {
        let file = work.join(name);
        std::fs::write(&file, messy).unwrap();
        m.open_file(&file);
        m.run_command(Cmd::FormatDocument);
        assert_eq!(
            m.active_editor_mut().unwrap().text(),
            clean,
            "{name}: {:?}",
            m.message
        );
    }

    m.format_on_save = true;
    let file = work.join("e.js");
    std::fs::write(&file, "let x = {a:1}\n").unwrap();
    open_dirty(&mut m, &file);
    assert_eq!(m.save_active(), SaveOutcome::Saved);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "let x = { a: 1 };\n"
    );

    let broken = work.join("broken.ts");
    std::fs::write(&broken, "const = ;\n").unwrap();
    open_dirty(&mut m, &broken);
    assert_eq!(m.save_active(), SaveOutcome::Saved);
    assert_eq!(std::fs::read_to_string(&broken).unwrap(), "const = ;\n\n");
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.starts_with("Prettier")),
        "{:?}",
        m.message
    );
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}
