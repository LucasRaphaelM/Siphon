document.addEventListener('DOMContentLoaded', () => {
  initTauriEvents();
  setupProgresso();
});


const { open } = window.__TAURI__.dialog;
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { basename, extname } = window.__TAURI__.path;
const { stat } = window.__TAURI__.fs;

const dropArea = document.getElementById("fileDropSearch");
const fileInput = document.getElementById("fileInput");
const fileList = document.getElementById("file-list");

// FUNÇÃO CHAMADA PARA COPIAR OS ARQUIVOS
async function processarArquivos(paths) {
  if (!paths || paths.length === 0) return;

  

  for (const path of paths) {
    try {
      // 1. Pega o nome do arquivo (ex: "video.mp4")
      const nomeCompleto = await basename(path);
      
      // 2. Pega a extensão (ex: "mp4")
      const extensao = await extname(path);

      const apenasNome = nomeCompleto.replace(`.${extensao}`, "");
      
      // 3. Pega os metadados para obter o tamanho
      const metadata = await stat(path);
      const tamanhoBytes = metadata.size;
      const tamanhoMB = (tamanhoBytes / (1024 * 1024)).toFixed(2); // Converte para MB

      // Printando separadamente no console
      console.log("--- Detalhes do Arquivo ---");
      console.log("Caminho:", path);
      console.log("Nome:", apenasNome);
      console.log("Formato:", extensao);
      console.log("Tamanho:", tamanhoMB + " MB");
      console.log("---------------------------");


      // Adiciona na lista visual com os detalhes
      //const li = document.createElement('li');
      //li.innerHTML = `<strong>${nomeCompleto}</strong> (${tamanhoMB} MB) - <small>${extensao}</small>`;
      //fileList.appendChild(li);
      fileList.innerHTML = `
        <div class="file-div">
          
          <div class="conteudo-file-div">
            <div class="logo-titulo-tipo">
              <div class="logo-arquivo-div">
                <i class="bi bi-filetype-mp4 tipo-arquivo-logo"></i>
              </div>
              <div class="titulo-tipo">
                <h4 class="title-file-card">${apenasNome}</h4>
                <p class="tamanho-tipo">${tamanhoMB} MB · ${extensao.toUpperCase()}</p>
              </div>
            </div>
            <div class="porcentagem-botoes">
              <p class="porcentagem" id="porcentagem">85%</p>
              <!-- <button type="button" class="btn btn-primary button-copy"><i class="bi bi-clipboard2 copy"></i></button>
              <button type="button" class="btn btn-primary button-delete"><i class="bi bi-trash delete"></i></button> -->
            </div>
          </div>
          <div class="progress-container">
            <div id="porcentagemBarra" class="progress-fill"></div>
          </div>

        </div>
      `; 

      // Inicia a cópia no Rust
      const resultado = await invoke("copiar_com_progresso", { caminhoOrigem: path });
      console.log("Salvo em:", resultado);

    } catch (err) {
      console.error("Erro ao processar detalhes do arquivo:", err);
    }
  }
}


// BROWSE FILES
dropArea.addEventListener("click", async () => {
  try {
    // Abre a janela nativa de seleção de arquivos
    const selected = await open({
      multiple: true, // Permite selecionar vários
      directory: false, // Queremos arquivos, não pastas
    });

    if (selected === null) {
      console.log("Seleção cancelada");
    } else {
      // O 'open' já retorna um array de caminhos (strings) ou uma string única
      const paths = Array.isArray(selected) ? selected : [selected];
      
      console.log("Manual Input Selection (Native):", paths);
      
      // Agora o path será o caminho real, ex: "C:\Users\Lucas\Downloads\musica.mp3"
      await processarArquivos(paths);
    }
  } catch (err) {
    console.error("Erro ao abrir seletor de arquivos:", err);
  }
});


// DRAG AND DROP
async function initTauriEvents() {
  // Listen para o DROP nativo
  await listen('tauri://drag-drop', async (event) => {
    const paths = event.payload.paths;
    console.log("Tauri Native Drop:", paths);
    
    await processarArquivos(paths);
    
    resetVisuals();
  });

  // Eventos de Hover (Entrar/Sair)
  await listen('tauri://drag-enter', () => {
    setHoverVisuals();
  });

  await listen('tauri://drag-leave', () => {
    resetVisuals();
  });
}

// --- 4. FEEDBACK VISUAL E PROGRESSO ---

function setHoverVisuals() {
  dropArea.classList.add("fileDropSearch-hover");
  document.getElementById("pastaIcone").classList.remove("bi-folder");
  document.getElementById("pastaIcone").classList.add("bi-folder2-open");
}

function resetVisuals() {
  dropArea.classList.remove("fileDropSearch-hover");
  document.getElementById("pastaIcone").classList.remove("bi-folder2-open");
  document.getElementById("pastaIcone").classList.add("bi-folder");
}


// Função para quando o mouse entra
document.getElementById('fileDropSearch').addEventListener('mouseenter', () => {
    document.getElementById('pastaIcone').classList.replace("bi-folder", "bi-folder2-open")
});

document.getElementById('fileDropSearch').addEventListener('mouseleave', () => {
    document.getElementById('pastaIcone').classList.replace("bi-folder2-open", "bi-folder")
});

async function setupProgresso() {
  await listen('progresso-copia', (event) => {
    const porcentagem = event.payload;
    
    const barra = document.getElementById('porcentagemBarra');
    const texto = document.getElementById('porcentagem');
    
    if (barra) barra.style.width = `${porcentagem}%`;
    if (texto) {
      texto.innerText = `${porcentagem}%`;

      // Se chegou em 100, espera 1 segundo e troca pelo ícone
      if (porcentagem === 100) {
        setTimeout(() => {
          // Usamos innerHTML porque você está inserindo uma tag <i>
          texto.innerHTML = `<i class="bi bi-check-lg" style="font-size: 24px; color: #00ff00"></i>`;
        }, 1000);
      }
    }
  });
}


// Prevenir comportamento padrão do browser
window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault());

// async function startServer() {
//   document.getElementById('btnOnOffIcon').classList.replace("power-off", "power-none");
//   document.getElementById('title-p').innerText = "Turning on server";


//   const menuPath = await invoke("get_menu_html_path");
//   const port = await invoke("start_local_server", { caminho: menuPath });
//   const fileUrl = `http://localhost:${port}`;

//   const url = await invoke("start_cloudflared_tunnel", { port: `${port}` });

//   document.getElementById('btnOnOffIcon').classList.replace("power-none", "power-on");
//   document.getElementById('title-p').innerText = "Server On";
//   document.getElementById('btnOnOff').onclick = stopServer;

//   setTimeout( () => {
//     document.getElementById('title-p').innerText = "Upload your files";
//   }, 2000);

//   console.log("LocalHost: ", fileUrl);
//   console.log("CloudFlared: ", url);
//   console.log("Menu Path:", menuPath)
// }


async function startServer() {
  document.getElementById('btnOnOffIcon').classList.replace("power-off", "power-none");
  document.getElementById('title-p').innerText = "Turning on server";

  const port = await invoke("start_local_server");

  const url = `http://localhost:${port}`;
  document.getElementById('teste-id').innerHTML= `http://localhost:${port}`;

  document.getElementById('btnOnOffIcon').classList.replace("power-none", "power-on");
  document.getElementById('title-p').innerText = "Server On";
  document.getElementById('btnOnOff').onclick = stopServer;

  setTimeout( () => {
    document.getElementById('title-p').innerText = "Upload your files";
  }, 2000);

  console.log(url)

}

async function stopServer() {
  document.getElementById('btnOnOffIcon').classList.replace("power-on", "power-off");
  document.getElementById('title-p').innerText = "Server off";
}

function teste1() {
  document.getElementById('teste-id').innerHTML="FUNCIONA 1";
}

function teste2() {
  document.getElementById('teste-id').innerHTML="FUNCIONA 2";
}

async function mudarEPropagar(novoConteudo) {
    // 1. Muda a sua tela local
    document.getElementById('teste-id').innerHTML = novoConteudo;

    // 2. Envia para o "Localhost" do Rust
    await invoke('atualizar_html_servidor', { novoHtml: document.documentElement.outerHTML });
}




window.addEventListener('message', (event) => {
    if (event.origin !== 'http://localhost:5500') return; // segurança

    if (event.data.action === 'updateText') {
      document.getElementById('testeId').innerText = event.data.value;
    }
  });