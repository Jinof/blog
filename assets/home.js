(() => {
  const canvas = document.getElementById('home-canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const ink = '#14171a';
  function line(a, b, width, color = ink) {
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.beginPath();
    ctx.moveTo(...a);
    ctx.lineTo(...b);
    ctx.stroke();
  }
  function rect(x, y, w, h, color) {
    ctx.fillStyle = color;
    ctx.fillRect(x - w / 2, y - h / 2, w, h);
  }
  function draw() {
    const { width, height } = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx.setTransform(dpr, 0, 0, -dpr, canvas.width / 2, canvas.height / 2);
    rect(0, -193, 150, 7, 'rgba(0,0,0,.12)');
    line([-310, -205], [310, -205], 3, 'rgba(20,23,26,.3)');
    line([0, -66], [0, 32], 7);
    const head = [[-13,107],[13,107],[26,81],[13,55],[-13,55],[-26,81]];
    head.forEach((p, i) => line(p, head[(i + 1) % head.length], 5.5));
    [[[0,32],[-38,-2]],[[-38,-2],[-72,-35]],[[0,32],[34,60]],[[34,60],[65,99]]]
      .forEach(([a,b]) => line(a,b,6));
    [[[0,-66],[-24,-123]],[[-24,-123],[-42,-184]],[[0,-66],[23,-121]],[[23,-121],[45,-184]]]
      .forEach(([a,b]) => line(a,b,6.5));
    ctx.save();
    ctx.translate(95, 118);
    ctx.rotate(.12);
    rect(0,-4,110,58,ink);
    rect(-26.5,2,47,42,'#f6f2e5');
    rect(26.5,2,47,42,'#f6f2e5');
    rect(0,2,5,44,ink);
    [[[-50,-20],[-50,24]],[[-50,-20],[-3,-20]],[[-50,24],[-3,24]],[[50,-20],[50,24]],[[3,-20],[50,-20]],[[3,24],[50,24]]]
      .forEach(([a,b]) => line(a,b,1.5,'rgba(20,23,26,.55)'));
    [16,10,4].forEach(y => line([12,y],[y === 4 ? 38 : 46,y],2,'rgba(20,23,26,.5)'));
    rect(1,30,3,16,ink);
    ctx.restore();
  }
  new ResizeObserver(draw).observe(canvas);
  window.addEventListener('resize', draw);
  draw();
})();
