const { invoke } = window.__TAURI__.core;

const CHUNK_SIZE = 256 * 256; // 5MB

async function sendFile(file) {
  const progressBar = document.getElementById("progressBar");

  const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
  let currentChunk = 0;

  for (let start = 0; start < file.size; start += CHUNK_SIZE) {
    const end = Math.min(start + CHUNK_SIZE, file.size);
    const blob = file.slice(start, end);

    const arrayBuffer = await blob.arrayBuffer();
    const uint8Array = Array.from(new Uint8Array(arrayBuffer));

    await invoke("upload_chunk", {
      fileName: file.name,
      chunk: uint8Array,
      isFirst: currentChunk === 0
    });

    currentChunk++;

    const progress = (currentChunk / totalChunks) * 100;
    progressBar.style.width = progress + "%";
  }
  
  console.log("Upload concluído");
  document.getElementById('concluido').innerHTML = `
    <p>UPLOAD CONCLUÍDO</p>
  `;
}

const fileInput = document.getElementById("fileInput");
const dropArea = document.getElementById("dropArea");

// Clique normal
fileInput.addEventListener("change", async (event) => {
  const file = event.target.files[0];
  if (file) {
    await sendFile(file);
  }
});

// Drag visual
dropArea.addEventListener("dragover", (e) => {
  e.preventDefault();
  dropArea.classList.add("dragover");
});

dropArea.addEventListener("dragleave", () => {
  dropArea.classList.remove("dragover");
});

// Drop
dropArea.addEventListener("drop", async (e) => {
  e.preventDefault();
  dropArea.classList.remove("dragover");

  const file = e.dataTransfer.files[0];
  if (file) {
    await sendFile(file);
  }
});