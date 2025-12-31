use std::sync::{Mutex, OnceLock};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    utils::platform::{target_triple, Target},
    App, AppHandle, Wry,
};
use tauri_plugin_shell::{process::CommandChild, ShellExt};

const TRAY_ID: &str = "tray";
const ACTION_ENABLE: &str = "Enable";
const ACTION_DISABLE: &str = "Disable";
const QUIT: &str = "Quit";
const ACTION_ID: &str = "action";
const QUIT_ID: &str = "quit";

static COMMAND: OnceLock<Mutex<Option<CommandChild>>> = OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if Target::from_triple(&target_triple()?) == Target::MacOS {
                build_tray(app)?;
                let _ = handle_action(app.handle());
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &mut App) -> Result<(), tauri::Error> {
    let menu = Menu::new(app)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon_as_template(true)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .menu(&build_menu(app.handle(), ACTION_DISABLE)?)
        .on_menu_event(|app, event| match event.id.as_ref() {
            QUIT_ID => {
                if let Some(lock) = COMMAND.get() {
                    if let Ok(mut data) = lock.lock() {
                        if let Some(child) = data.take() {
                            let _ = child.kill();
                        }
                    }
                }
                app.exit(0);
            }
            ACTION_ID => {
                let _ = handle_action(app);
            }
            _ => unreachable!(),
        })
        .build(app)?;
    Ok(())
}

fn rebuild_menu(app: &AppHandle, action: &str) -> Result<(), tauri::Error> {
    let menu = build_menu(app, action)?;
    app.tray_by_id(TRAY_ID).unwrap().set_menu(Some(menu))?;
    Ok(())
}
fn build_menu(app: &AppHandle, action: &str) -> Result<Menu<Wry>, tauri::Error> {
    let quit_i = MenuItem::with_id(app, QUIT_ID, QUIT, true, None::<&str>)?;
    let action = MenuItem::with_id(app, ACTION_ID, action, true, None::<&str>)?;
    Menu::with_items(app, &[&action, &quit_i])
}

fn handle_action(app: &AppHandle) -> Result<(), tauri_plugin_shell::Error> {
    match COMMAND.get() {
        Some(lock) => {
            if let Ok(mut data) = lock.lock() {
                if data.is_none() {
                    if let Ok(process) = spawn_caffeinate(app) {
                        *data = Some(process);
                        let _ = rebuild_menu(app, ACTION_DISABLE);
                    }
                } else {
                    data.take().unwrap().kill()?;
                    let _ = rebuild_menu(app, ACTION_ENABLE);
                }
            }
        }

        //Only at the application startup COMMAND is empty
        None => {
            if let Ok(process) = spawn_caffeinate(app) {
                COMMAND.get_or_init(|| Mutex::new(Some(process)));
            }
        }
    }
    Ok(())
}

fn spawn_caffeinate(app: &AppHandle) -> Result<CommandChild, tauri_plugin_shell::Error> {
    let (_, child) = app.shell().command("caffeinate").args(["-d"]).spawn()?;
    println!("Caffeinate pid: {}", child.pid());
    Ok(child)
}
