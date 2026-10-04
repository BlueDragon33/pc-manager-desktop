export type PageId =
  | "overview"
  | "health"
  | "clean"
  | "startup"
  | "apps"
  | "storage"
  | "duplicates"
  | "monitor"
  | "restore"
  | "settings";

export type NavItem = {
  id: PageId;
  label: string;
  shortLabel: string;
  description: string;
  icon: string;
};

export const NAV_ITEMS: NavItem[] = [
  {
    id: "overview",
    label: "Overview",
    shortLabel: "Home",
    description: "A calm summary of PC Manager status and next actions.",
    icon: "⌂",
  },
  {
    id: "health",
    label: "Health Check",
    shortLabel: "Health",
    description: "Review storage, performance, security, updates, and privacy findings.",
    icon: "♡",
  },
  {
    id: "clean",
    label: "Smart Clean",
    shortLabel: "Clean",
    description: "Preview high-confidence cleanup candidates before anything is removed.",
    icon: "✦",
  },
  {
    id: "startup",
    label: "Startup",
    shortLabel: "Startup",
    description: "Review programs that start with Windows and their measured impact.",
    icon: "↗",
  },
  {
    id: "apps",
    label: "Apps",
    shortLabel: "Apps",
    description: "Inspect installed applications and use standard uninstall flows.",
    icon: "▦",
  },
  {
    id: "storage",
    label: "Storage",
    shortLabel: "Storage",
    description: "Understand which folders and file groups consume disk capacity.",
    icon: "◫",
  },
  {
    id: "duplicates",
    label: "Duplicates",
    shortLabel: "Dupes",
    description: "Find byte-identical files with staged hashing and manual selection.",
    icon: "≡",
  },
  {
    id: "monitor",
    label: "Monitor",
    shortLabel: "Monitor",
    description: "View low-overhead CPU, memory, disk, network, and process activity.",
    icon: "⌁",
  },
  {
    id: "restore",
    label: "Restore",
    shortLabel: "Restore",
    description: "Review operation history and restore changes that support rollback.",
    icon: "↶",
  },
  {
    id: "settings",
    label: "Settings",
    shortLabel: "Settings",
    description: "Configure appearance and local application preferences.",
    icon: "⚙",
  },
];

export function getNavItem(id: PageId): NavItem {
  const item = NAV_ITEMS.find((entry) => entry.id === id);

  if (!item) {
    throw new Error(`Unknown page: ${id satisfies never}`);
  }

  return item;
}

export function isPageId(value: string): value is PageId {
  return NAV_ITEMS.some((item) => item.id === value);
}
