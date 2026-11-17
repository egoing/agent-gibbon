window.showDirectionGuide = restoring => {
  document.body.classList.toggle('restoring', restoring);
  document.querySelector('title').textContent =
    `위로 메모 모아보기, 오른쪽으로 오른쪽 정렬, 아래로 전체 ${restoring ? '복원' : '숨기기'}, 왼쪽으로 왼쪽 정렬, Esc 키로 취소`;
  document.body.classList.remove('closing');
  document.body.classList.add('open');
};
