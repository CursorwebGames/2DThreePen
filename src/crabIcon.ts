export function crabIcon(x: number, y: number) {
    fill(255);
    ellipse(x, y, 50, 30);

    // eyes
    fill(0);
    circle(x - 10, y - 15, 5);
    circle(x + 10, y - 15, 5);


}

function roundedTrapezoid(
    x: number,
    y: number,
    topW: number,
    bottomW: number,
    h: number,
    r: number
) {
    const topLeft = x - topW / 2;
    const topRight = x + topW / 2;
    const bottomLeft = x - bottomW / 2;
    const bottomRight = x + bottomW / 2;

    beginShape();

    // Top-left rounded corner
    vertex(topLeft + r, y);
    quadraticVertex(topLeft, y, topLeft, y + r);

    // Left slanted edge
    vertex(bottomLeft, y + h - r);

    // Bottom-left rounded corner
    quadraticVertex(bottomLeft, y + h, bottomLeft + r, y + h);

    // Bottom edge
    vertex(bottomRight - r, y + h);

    // Bottom-right rounded corner
    quadraticVertex(bottomRight, y + h, bottomRight, y + h - r);

    // Right slanted edge
    vertex(topRight, y + r);

    // Top-right rounded corner
    quadraticVertex(topRight, y, topRight - r, y);

    endShape(CLOSE);
}