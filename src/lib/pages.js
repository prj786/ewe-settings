// The sections of ewe Settings, in nav order, in the four groups of the
// side navigation (Side navigation card: group headings). The label is the
// nav item AND the page's title; they never differ. `id` is the pane id the
// rest of the app (and Ctrl+1…9, the search, dev-mock.html) uses — kept from
// before the regrouping so nothing that names a pane breaks.

export const NAV = [
  {
    title: "Look",
    pages: [
      { id: "appearance", label: "Appearance", icon: "palette" },
      { id: "wallpaper", label: "Wallpaper", icon: "image" },
      { id: "animations", label: "Animations", icon: "gauge" },
      { id: "accessibility", label: "Accessibility", icon: "accessibility" }
    ]
  },
  {
    title: "Desktop",
    pages: [
      { id: "layout", label: "Layout", icon: "layout" },
      { id: "windowrules", label: "Window rules", icon: "windowRules" },
      { id: "displays", label: "Displays", icon: "monitor" },
      { id: "startup", label: "Startup apps", icon: "rocket" },
      { id: "shortcuts", label: "Keyboard shortcuts", icon: "command" }
    ]
  },
  {
    title: "Devices",
    pages: [
      { id: "network", label: "Network", icon: "wifi" },
      { id: "bluetooth", label: "Bluetooth", icon: "bluetooth" },
      { id: "input", label: "Keyboard and mouse", icon: "keyboard" },
      { id: "power", label: "Power", icon: "power" },
      { id: "saver", label: "Screensaver", icon: "saver" }
    ]
  },
  {
    title: "System",
    pages: [
      { id: "defaults", label: "Default apps", icon: "apps" },
      { id: "time", label: "Time and place", icon: "globe" },
      { id: "user", label: "User", icon: "user" },
      { id: "system", label: "About this computer", icon: "cpu" }
    ]
  }
];

/** Every page in nav order (Up/Down and Ctrl+1…9 walk this). */
export const PAGES = NAV.flatMap((g) => g.pages.map((p) => ({ ...p, group: g.title })));

export const pageById = (id) => PAGES.find((p) => p.id === id);
