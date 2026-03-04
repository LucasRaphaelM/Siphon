use std::fs::{self, create_dir_all, OpenOptions}; // Adicionado self e fs aqui
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{LogicalPosition, Manager};

// Imports necessários para o Sidecar e Canais na V2
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::oneshot;

#[tauri::command]
async fn start_cloudflared_tunnel(app: tauri::AppHandle, port: String) -> Result<String, String> {
    // Na V2, usamos o app.shell().sidecar()
    let (mut rx, _child) = app
        .shell()
        .sidecar("cloudflared")
        .map_err(|e| e.to_string())?
        .args(["tunnel", "--url", &format!("http://localhost:{}", port)])
        .spawn()
        .map_err(|e| e.to_string())?;

    let (tx, rx_link) = oneshot::channel::<String>();
    let mut tx_opt = Some(tx);

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stderr(line_bytes) = event {
                let line = String::from_utf8_lossy(&line_bytes);
                println!("Cloudflared Log: {}", line); // Para você ver no console

                // Procura por qualquer link que contenha .trycloudflare.com
                if line.contains("https://") && line.contains(".trycloudflare.com") {
                    // Tenta isolar a URL limpando caracteres residuais
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.starts_with("https://") && part.contains(".trycloudflare.com") {
                            if let Some(tx) = tx_opt.take() {
                                let _ = tx.send(part.to_string());
                            }
                        }
                    }
                }
            }
        }
    });

    match tokio::time::timeout(std::time::Duration::from_secs(10), rx_link).await {
        Ok(Ok(url)) => Ok(url),
        Ok(Err(_)) => Err("Canal de comunicação fechado".into()),
        Err(_) => Err("Tempo esgotado ao tentar gerar o link do túnel".into()),
    }
}

#[tauri::command]
fn start_local_server(caminho: String) -> Result<u16, String> {
    #[cfg(windows)]
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "cloudflared-x86_64-pc-windows-msvc.exe"])
        .output();
    // Tenta abrir o listener na porta 0 (o SO escolhe uma disponível)
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Erro ao criar listener: {}", e))?;

    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    // Clonamos o path para mover para dentro da thread
    let path_to_serve = caminho.clone();

    thread::spawn(move || {
        // Aceita as conexões que chegarem
        for mut stream in listener.incoming().flatten() {
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);

            // Tenta ler o arquivo solicitado no parâmetro
            match fs::read(&path_to_serve) {
                Ok(content) => {
                    // Monta o cabeçalho HTTP de sucesso
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                        content.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&content);
                }
                Err(e) => {
                    // Caso o arquivo não exista ou ocorra erro de leitura
                    let error_msg = format!("Erro ao ler arquivo: {}", e);
                    let header = format!(
                        "HTTP/1.1 404 NOT FOUND\r\nContent-Length: {}\r\n\r\n",
                        error_msg.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(error_msg.as_bytes());
                }
            }
            let _ = stream.flush();
        }
    });

    Ok(port)
}

#[tauri::command]
fn upload_chunk(
    app: tauri::AppHandle,
    file_name: String,
    chunk: Vec<u8>,
    is_first: bool,
) -> Result<String, String> {
    // 👈 MUDOU AQUI

    let base_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;

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

    Ok(file_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            upload_chunk,
            start_local_server,
            start_cloudflared_tunnel,
            copiar_com_progresso
        ])
        .setup(|app| {
            // 1. Criamos os itens do menu
            let toggle_i =
                MenuItem::with_id(app, "toggle_visibility", "Mostrar", true, None::<&str>)?;
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
                    } = event
                    {
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

                                    let x = logical_size.width - 500.0 - 100.0;
                                    let y = logical_size.height - 350.0 - 200.0;
                                    let _ = window.set_position(tauri::Position::Logical(
                                        LogicalPosition { x, y },
                                    ));
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
                                    let x = logical_size.width - 500.0 - 100.0;
                                    let y = logical_size.height - 350.0 - 200.0;
                                    let _ = window.set_position(tauri::Position::Logical(
                                        LogicalPosition { x, y },
                                    ));
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


use std::fs::{File};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Runtime};
#[tauri::command]
async fn copiar_com_progresso<R: Runtime>(
    app: AppHandle<R>,
    caminho_origem: String,
) -> Result<String, String> {
    // 1. Pega o caminho base do sistema (Roaming no Windows)
    let mut caminho_destino = app.path().config_dir()
        .map_err(|e| e.to_string())?;
    
    // 2. Constrói o caminho manualmente: Roaming -> Siphon -> Files
    caminho_destino.push("Siphon");
    caminho_destino.push("Files");

    // 3. Cria as pastas (Siphon e Files dentro dela)
    fs::create_dir_all(&caminho_destino).map_err(|e| e.to_string())?;

    let origem_path = std::path::PathBuf::from(&caminho_origem);
    let nome_arquivo = origem_path.file_name().ok_or("Nome inválido")?;
    
    // Caminho final: AppData/Roaming/Siphon/Files/nome_do_arquivo.ext
    caminho_destino.push(nome_arquivo);

    // ... restante do código de cópia (File::open, buffer, etc) ...
    
    // Copiando para facilitar:
    let mut arquivo_origem = File::open(&origem_path).map_err(|e| e.to_string())?;
    let total_size = arquivo_origem.metadata().map_err(|e| e.to_string())?.len();
    let mut arquivo_destino = File::create(&caminho_destino).map_err(|e| e.to_string())?;

    let mut buffer = [0; 64 * 1024];
    let mut bytes_copiados = 0;

    while let Ok(n) = arquivo_origem.read(&mut buffer) {
        if n == 0 { break; }
        arquivo_destino.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        bytes_copiados += n as u64;
        let porcentagem = (bytes_copiados as f64 / total_size as f64 * 100.0) as u64;
        app.emit("progresso-copia", porcentagem).unwrap();
    }

    Ok(caminho_destino.to_string_lossy().to_string())
}