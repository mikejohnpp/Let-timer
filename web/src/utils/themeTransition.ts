/**
 * withThemeTransition: adds a temporary 'theme-transition' class to <html> for 250ms
 * so background, text and border colors smoothly interpolate without permanent CSS transitions.
 * Ref: docs/motion_plan.md A28
 */
export function withThemeTransition(fn: () => void): void {
  if (typeof document === 'undefined') {
    fn();
    return;
  }

  const root = document.documentElement;
  root.classList.add('theme-transition');
  fn();

  window.setTimeout(() => {
    root.classList.remove('theme-transition');
  }, 300);
}

