import "p5";

window.setup = () => {
  const canvas = document.querySelector("canvas")!;
  createCanvas(500, 500, canvas);
};

window.draw = () => {
  background('red');
};