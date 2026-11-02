import { SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";

let _isThisWebviewFocused = $state(false);
subscribe(SeelenEvent.GlobalFocusChanged, ({ payload: { hwnd, ownerHwnd } }) => {
  _isThisWebviewFocused = Widget.self.windowId === hwnd || Widget.self.windowId === ownerHwnd;
});

export const isThisWebviewFocused = {
  get value() {
    return _isThisWebviewFocused;
  },
};

// The media query follows the input devices Windows found at boot. When the mouse shows up late (a USB receiver or
// a KVM switch, waking from sleep) Windows keeps reporting no fine pointer until the next reboot, and the dock and the
// toolbar, which never auto-hide on touch, stay up for good. The pointer that actually moves over the widget wins.
const _pointerQuery = window.matchMedia("(hover: hover) and (pointer: fine)");
let _isTouchPrimary = $state(!_pointerQuery.matches);
_pointerQuery.addEventListener("change", (e) => {
  _isTouchPrimary = !e.matches;
});
function onPointer(e: PointerEvent) {
  _isTouchPrimary = e.pointerType === "touch";
}
window.addEventListener("pointermove", onPointer, { passive: true, capture: true });
window.addEventListener("pointerdown", onPointer, { passive: true, capture: true });

export const isTouchPrimary = {
  get value() {
    return _isTouchPrimary;
  },
};
