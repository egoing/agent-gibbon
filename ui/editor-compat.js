// System WebKit on older supported macOS may predate Array.prototype.at.
if (!Array.prototype.at) Object.defineProperty(Array.prototype, 'at', {
  configurable: true, writable: true,
  value(index) {
    const value = Number(index) || 0;
    const offset = value < 0 ? Math.ceil(value) : Math.floor(value);
    return this[offset < 0 ? this.length + offset : offset];
  },
});
