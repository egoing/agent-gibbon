// Rust supplies a cubic between facing ports with parallel, axis-aligned handles.
window.relationCurve = (points, arrow, scale, marker) => {
  const [start, first, , end] = points;
  const count = arrow === 'both' ? 2 : arrow === 'none' ? 0 : 1;
  let shaft = points;
  if (count) {
    // At exact contact both handles collapse. The cubic then follows the chord.
    const collapsed = first[0] === start[0] && first[1] === start[1];
    const directionPoint = collapsed ? end : first;
    const axis = directionPoint[0] === start[0] ? 1 : 0;
    const span = end[axis] - start[axis];
    if (!span) return { d: '', center: start };

    // Each standard head is seven stroke widths long. Keep room for a shaft
    // tangent when opposing ports are closer than the head(s) plus one pixel.
    const gap = Math.abs(span);
    const stroke = 2 * scale;
    const length = Math.min(7 * stroke, (gap - Math.min(scale, gap / 2)) / count);
    marker.setAttribute('markerWidth', length / stroke);
    marker.setAttribute('markerHeight', length / stroke);
    const a = [...start];
    const d = [...end];
    const inset = Math.sign(span) * length;
    if (count === 2) a[axis] += inset;
    d[axis] -= inset;

    // Use the same half-gap handles as Rust between the two visible bases.
    const b = [...a];
    const c = [...d];
    b[axis] = c[axis] = (a[axis] + d[axis]) / 2;
    shaft = [a, b, c, d];
  }
  const [a, b, c, d] = shaft;
  return {
    d: `M${a} C${b} ${c} ${d}`,
    center: [0, 1].map(i => (a[i] + 3 * b[i] + 3 * c[i] + d[i]) / 8),
  };
};
