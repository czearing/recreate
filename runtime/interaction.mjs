export const closedInteraction = Object.freeze({
  openSurface: null,
  activeTrigger: null,
});

export function reduceInteraction(state, event) {
  if (event.type === 'activate' && !event.stateful) {
    return [state, { type: 'invoke', surface: event.surface, trigger: event.trigger }];
  }
  if (event.type === 'activate' && !event.closable) {
    return [
      { openSurface: event.surface, activeTrigger: event.trigger },
      { type: 'open', surface: event.surface, trigger: event.trigger },
    ];
  }
  if (event.type === 'activate') {
    if (state.openSurface === event.surface && state.activeTrigger === event.trigger) {
      return [closedInteraction, { type: 'close', restoreTrigger: event.trigger }];
    }
    return [
      { openSurface: event.surface, activeTrigger: event.trigger },
      { type: 'open', surface: event.surface, trigger: event.trigger },
    ];
  }
  if ((event.type === 'escape' || event.type === 'outside') && state.openSurface !== null) {
    return [closedInteraction, { type: 'close', restoreTrigger: state.activeTrigger }];
  }
  return [state, { type: 'none' }];
}
