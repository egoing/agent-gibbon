const button = document.querySelector('button');
window.startCountdown = () => {
  button.classList.remove('running');
  void button.offsetWidth;
  button.classList.add('running');
};
button.onclick = () => window.__TAURI__.core.invoke('undo_arrangement');
