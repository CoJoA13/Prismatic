// SPDX-License-Identifier: GPL-3.0-or-later

let found = false;
for (const existingPanel of panels()) {
    if (existingPanel.widgets("io.github.CoJoA13.Prismatic").length > 0) {
        found = true;
        break;
    }
}

if (!found) {
    const panel = new Panel;
    panel.location = "bottom";
    panel.lengthMode = "fit";
    panel.alignment = "center";
    panel.height = 64;
    panel.hiding = "dodgewindows";
    const widget = panel.addWidget("io.github.CoJoA13.Prismatic");
    widget.currentConfigGroup = ["General"];
    widget.writeConfig("ownedPanelId", String(panel.id));
    widget.reloadConfig();
}
