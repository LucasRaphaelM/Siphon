use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{LogicalPosition, Manager};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;

#[tauri::command]
fn upload_chunk(
    app: tauri::AppHandle,
    file_name: String,
    chunk: Vec<u8>,
    is_first: bool,
) -> Result<(), String> {

    let base_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;

    let roaming_dir = base_dir.parent().ok_or("Erro AppData")?;
    let siphon_dir = roaming_dir.join("Siphon");
    let files_dir = siphon_dir.join("Files");

    if !files_dir.exists() {
        create_dir_all(&files_dir).map_err(|e| e.to_string())?;
    }

    let file_path = files_dir.join(&file_name);

    let mut file = if is_first {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&file_path)
            .map_err(|e| e.to_string())?
    } else {
        OpenOptions::new()
            .append(true)
            .open(&file_path)
            .map_err(|e| e.to_string())?
    };

    file.write_all(&chunk).map_err(|e| e.to_string())?;

    Ok(())
}



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![upload_chunk])
        .setup(|app| {
            // 1. Criamos os itens do menu
            let toggle_i = MenuItem::with_id(app, "toggle_visibility", "Mostrar", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_i, &quit_i])?;

            // 2. Criamos DOIS clones com o nome que você quer usar
            let toggle_i_clone_tray = toggle_i.clone();
            let toggle_i_clone_menu = toggle_i.clone();

            // 3. Configuração do Tray
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            
                            if is_visible {
                                window.hide().unwrap();
                                toggle_i_clone_tray.set_text("Mostrar").unwrap();
                            } else {
                                // Lógica de Reset de Posição
                                if let Some(monitor) = window.primary_monitor().unwrap_or(None) {
                                    let monitor_size = monitor.size();
                                    let scale_factor = monitor.scale_factor();
                                    let logical_size = monitor_size.to_logical::<f64>(scale_factor);
                                    
                                    let x = logical_size.width - 400.0 - 200.0;
                                    let y = logical_size.height - 500.0 - 60.0;
                                    let _ = window.set_position(tauri::Position::Logical(LogicalPosition { x, y }));
                                }
                                window.show().unwrap();
                                window.set_focus().unwrap();
                                toggle_i_clone_tray.set_text("Esconder").unwrap();
                            }
                        }
                    }
                })
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle_visibility" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                window.hide().unwrap();
                                toggle_i_clone_menu.set_text("Mostrar").unwrap();
                            } else {
                                // Reset de posição também no clique do menu
                                if let Some(monitor) = window.primary_monitor().unwrap_or(None) {
                                    let monitor_size = monitor.size();
                                    let scale_factor = monitor.scale_factor();
                                    let logical_size = monitor_size.to_logical::<f64>(scale_factor);
                                    let x = logical_size.width - 400.0 - 200.0;
                                    let y = logical_size.height - 500.0 - 60.0;
                                    let _ = window.set_position(tauri::Position::Logical(LogicalPosition { x, y }));
                                }
                                window.show().unwrap();
                                window.set_focus().unwrap();
                                toggle_i_clone_menu.set_text("Esconder").unwrap();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}