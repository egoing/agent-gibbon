window.setStashTip = (visible, shortcut) => {
  window.jajanLanguage.bind(document.getElementById('tip'), 'textContent',
    visible ? '복원: {shortcut} 놓고 ↓' : '', {shortcut});
};
