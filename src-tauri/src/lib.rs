use std::fs::{self, create_dir_all, OpenOptions}; // Adicionado self e fs aqui
use std::io::{Read, Write};
use std::path::Path;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{LogicalPosition, Manager};
use tiny_http::{Server, Response, Header};
use std::fs::{File};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::oneshot;
use std::thread;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::net::TcpListener;
use tauri::State;


static SERVER_RUNNING: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));


// Arquivos embutidos para build
#[cfg(not(debug_assertions))] // build
const INDEX_HTML: &str = include_str!("../../src/index.html");
#[cfg(not(debug_assertions))]
const STYLE_CSS: &str = include_str!("../../src/styles.css");
#[cfg(not(debug_assertions))]
const MAIN_JS: &str = include_str!("../../src/main.js");

fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

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

struct AppState {
    html_atual: Mutex<String>,
}

#[tauri::command]
fn atualizar_html_servidor(novo_html: String, state: State<'_, AppState>) {
    let mut conteudo = state.html_atual.lock().unwrap();
    *conteudo = novo_html; // Atualiza o HTML na memória do servidor
}

#[tauri::command]
fn start_local_server() -> Result<u16, String> {

    let mut running = SERVER_RUNNING.lock().unwrap();
    if *running {
        return Err("Servidor já está rodando".into());
    }

    let port = get_free_port();
    let server = Server::http(format!("127.0.0.1:{}", port))
    .map_err(|e| e.to_string())?;

    println!("Servidor rodando na porta {}", port);

    *running = true;

    thread::spawn(move || {
        for request in server.incoming_requests() {

            // Para DEV, lê do disco (atualizações imediatas)
            #[cfg(debug_assertions)]
            let html = {
                let exe_path = std::env::current_dir().unwrap();
                let html_path = exe_path.join("..").join("src").join("index.html");
                std::fs::read_to_string(html_path)
                    .unwrap_or_else(|_| "Erro ao carregar HTML".to_string())
            };

            // Para BUILD, usa embutido
            #[cfg(not(debug_assertions))]
            let html = INDEX_HTML;

            let response = Response::from_string(html)
                .with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap()
                );

            let _ = request.respond(response);
        }
    });

    Ok(port)
}

// #[tauri::command]
// fn stop_local_server() -> Result<String, String> {

//     let mut running = SERVER_RUNNING.lock().unwrap();

//     if *running {
//         *running = false;
//         Ok("Servidor parado".into())
//     } else {
//         Err("Servidor não está rodando".into())
//     }
// }


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            start_local_server,
            start_cloudflared_tunnel,
            copiar_com_progresso,
            atualizar_html_servidor,
        ])
        .manage(AppState {
            html_atual: Mutex::new("<h1>Iniciado</h1>".to_string()),
        })
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

                                    let x = logical_size.width - 350.0 - 60.0;
                                    let y = logical_size.height - 700.0 - 100.0;
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
                                    let x = logical_size.width - 350.0 - 60.0;
                                    let y = logical_size.height - 700.0 - 100.0;
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



fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    }

    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}