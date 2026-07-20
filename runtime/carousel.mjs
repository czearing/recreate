export function moveCarousel(state, direction) {
  const extent = Math.max(0, state.extent);
  const offset = direction === 'forward' ? extent : 0;
  return {
    ...state,
    offset,
    previousDisabled: offset === 0,
    nextDisabled: offset === extent,
  };
}
