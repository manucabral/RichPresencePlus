/** Whether the window is on screen, as the desktop side last said. */
export interface Screen {
  /** Polls run only while this is true. */
  readonly on: boolean;
  /** Records the new state and tells whether the window just came back. */
  set(visible: boolean): boolean;
}

/** A window counts as on screen until told otherwise. */
export function createScreen(): Screen {
  let on = true;
  return {
    get on() {
      return on;
    },
    set(visible) {
      const back = visible && !on;
      on = visible;
      return back;
    },
  };
}
