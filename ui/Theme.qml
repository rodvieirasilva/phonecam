pragma Singleton
import QtQuick

QtObject {
    id: theme

    // Current active theme colors
    property string accent: "#FFD60A"
    property string background: "#18181b"
    property string cardBackground: "#27272a"
    property string cardBorder: "#3f3f46"
    property string inputBackground: "#1e1e24"
    property string textPrimary: "#f4f4f5"
    property string textSecondary: "#a1a1aa"
    property string success: "#10b981"
    property string danger: "#ef4444"
    property string warning: "#f59e0b"

    // Palette presets for live theme switching
    readonly property var palettePresets: [
        { name: "Omarchy Gold",  hex: "#FFD60A" },
        { name: "Cyber Emerald", hex: "#10B981" },
        { name: "Sky Blue",      hex: "#38BDF8" },
        { name: "Neon Violet",   hex: "#A855F7" },
        { name: "Sunset Orange", hex: "#F97316" },
        { name: "Rose Pink",     hex: "#F43F5E" }
    ]

    function setAccent(colorHex) {
        accent = colorHex;
    }
}
