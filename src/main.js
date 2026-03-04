const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
// const CHUNK_SIZE = 256 * 256; // 5MB

// async function sendFile(file) {
//   const progressBar = document.getElementById("progressBar");

//   const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
//   let currentChunk = 0;
//   var caminho = "";
//   for (let start = 0; start < file.size; start += CHUNK_SIZE) {
//     const end = Math.min(start + CHUNK_SIZE, file.size);
//     const blob = file.slice(start, end);

//     const arrayBuffer = await blob.arrayBuffer();
//     const uint8Array = Array.from(new Uint8Array(arrayBuffer));

//      caminho =  await invoke("upload_chunk", {
//       fileName: file.name,
//       chunk: uint8Array,
//       isFirst: currentChunk === 0
//     });

//     currentChunk++;

//     const progress = (currentChunk / totalChunks) * 100;
//     progressBar.style.width = progress + "%";
//   }
  
//   console.log("Upload concluído");
//   document.getElementById('concluido').innerHTML = `
//     <p>UPLOAD CONCLUÍDO</p>
//   `;
//   const porta = await invoke('start_local_server', {caminho});
//   const fileUrl = `http://localhost:${porta}`;

//   const url = await invoke('start_cloudflared_tunnel', { port: '5500'});

//   console.log(caminho);
//   document.getElementById('arquivos-pasta').innerHTML= `
//     <p>${caminho}</p>
//     <p>${fileUrl}</p>
//     <p>${url}</p>
//   `;

// }

// const fileInput = document.getElementById("fileInput");
// const dropArea = document.getElementById("dropArea");

// // Clique normal
// fileInput.addEventListener("change", async (event) => {
//   const file = event.target.files[0];
//   if (file) {
//     await sendFile(file);
//   }
// });

// // Drag visual
// dropArea.addEventListener("dragover", (e) => {
//   e.preventDefault();
//   dropArea.classList.add("dragover");
// });

// dropArea.addEventListener("dragleave", () => {
//   dropArea.classList.remove("dragover");
// });

// // Drop
// dropArea.addEventListener("drop", async (e) => {
//   e.preventDefault();
//   dropArea.classList.remove("dragover");

//   const file = e.dataTransfer.files[0];
//   if (file) {
//     await sendFile(file);
//   }
// });

document.getElementById("fileDropSearch").addEventListener("mouseenter", () => {
  document.getElementById("pastaIcone").classList.remove("bi-folder");
  document.getElementById("pastaIcone").classList.add("bi-folder2-open");
  console.log("TESTE");
});

document.getElementById("fileDropSearch").addEventListener("mouseleave", () => {
  document.getElementById("pastaIcone").classList.remove("bi-folder2-open");
  document.getElementById("pastaIcone").classList.add("bi-folder");
});

const dropArea = document.getElementById("fileDropSearch");
const fileInput = document.getElementById("fileInput");
const fileList = document.getElementById("file-list");

// --- 1. THE UNIFIED HANDLER ---
// This handles both Tauri paths (strings) and Browser Files (objects)
function handleFiles(files) {
  if (!files || files.length === 0) return;

  // Clear previous list if needed
  fileList.innerHTML = '';

  for (const file of files) {
    const li = document.createElement('li');
    
    // If it's a string, it's a Tauri Path. If it has a .name, it's a Web File.
    const displayName = typeof file === 'string' ? file : file.name;
    
    li.textContent = displayName;
    fileList.appendChild(li);
    
    console.log("Processing:", displayName);
  }
}

// --- 2. BROWSER/MANUAL SELECTION ---

// Click area to open file explorer
dropArea.addEventListener("click", () => {
  fileInput.click();
});

// File selected via explorer button
fileInput.addEventListener("change", () => {
  handleFiles(fileInput.files);
});

// Standard web events to allow visual "hover" effects
dropArea.addEventListener("dragover", (e) => {
  e.preventDefault();
  dropArea.classList.add("hover");
});

dropArea.addEventListener("dragleave", () => {
  dropArea.classList.remove("hover");
});

// Standard web drop (fallback/internal)
dropArea.addEventListener("drop", (e) => {
  e.preventDefault();
  dropArea.classList.remove("hover");
  handleFiles(e.dataTransfer.files);
});

// --- 3. TAURI NATIVE DRAG & DROP ---

async function initTauriEvents() {
  // Listen for the NATIVE OS drop event
  await listen('tauri://drag-drop', (event) => {
    // Tauri gives us: { paths: string[], position: { x, y } }
    const paths = event.payload.paths;
    console.log("Tauri Native Drop:", paths);
    handleFiles(paths);
    dropArea.classList.remove("hover");
  });

  // Native drag enter
  await listen('tauri://drag-enter', () => {
    dropArea.classList.add("hover");
  });

  // Native drag leave
  await listen('tauri://drag-leave', () => {
    dropArea.classList.remove("hover");
  });
}

// Global prevent to stop browser from opening dropped files
window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault());

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  initTauriEvents();
});

async function setupCopia() {
  // Escuta a porcentagem vindo do Rust
  await listen('progresso-copia', (event) => {
    const porcentagem = event.payload;
    console.log(`Progresso: ${porcentagem}%`);
    document.getElementById("porcentagemEscrita").innerHTML = `
      <p>${porcentagem}%</p>
    `;
    
    // Atualize sua barra de progresso aqui
    document.getElementById('barra-progresso').style.width = `${porcentagem}%`;
    document.getElementById('texto-porcentagem').innerText = `${porcentagem}%`;
  });

  // Gatilho do Drag and Drop
  await listen('tauri://drag-drop', async (event) => {
    const paths = event.payload.paths;
    for (const path of paths) {
      try {
        const resultado = await invoke("copiar_com_progresso", { caminhoOrigem: path });
        console.log("Salvo em:", resultado);
      } catch (err) {
        console.error("Erro na cópia:", err);
      }
    }
  });
}

setupCopia();