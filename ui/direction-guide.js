window.showDirectionGuide = restoring => {
  const name = restoring ? 'direction-guide-restore' : 'direction-guide';
  const image = document.querySelector('img');
  document.querySelector('source').srcset = `${name}-still.png`;
  image.alt = `위로 바둑판 정렬, 오른쪽으로 오른쪽 정렬, 아래로 전체 ${restoring ? '복원' : '숨기기'}, 왼쪽 이동 또는 Esc 키로 취소`;
  document.body.classList.remove('closing');
  image.src = `${name}.svg?open=${performance.now()}`;
};
