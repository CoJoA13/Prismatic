// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

FocusScope {
    id: button

    required property int taskIndex
    required property var tasksSource
    required property var model
    readonly property var taskModel: model
    required property int iconSize
    required property color accentColor
    property var desktopActions: []
    readonly property string desktopId: normalizedDesktopId(taskModel.LauncherUrl || taskModel.AppId || "")
    readonly property bool pinned: Boolean(taskModel.IsLauncher || taskModel.HasLauncher)
    readonly property int windowCount: Number(taskModel.ChildCount || (taskModel.IsWindow ? 1 : 0))
    signal activateRequested()
    signal newInstanceRequested()
    signal closeRequested()
    signal pinRequested(string desktopId)
    signal unpinRequested(string desktopId)
    signal moveRequested(string desktopId, string beforeId)
    signal menuRequested(string desktopId)
    signal desktopActionRequested(string desktopId, string actionId)

    implicitWidth: iconSize + Kirigami.Units.smallSpacing * 2
    implicitHeight: iconSize + Kirigami.Units.smallSpacing * 2
    activeFocusOnTab: true
    Accessible.name: taskModel.Display || desktopId
    Accessible.description: i18np("%1 running window", "%1 running windows", windowCount)
    Accessible.role: Accessible.Button
    Drag.active: dragHandler.active
    Drag.source: button
    Drag.hotSpot.x: width / 2
    Drag.hotSpot.y: height / 2
    opacity: dragHandler.active ? 0.6 : 1

    Rectangle {
        anchors.fill: parent
        radius: Kirigami.Units.cornerRadius
        color: button.activeFocus || hover.hovered
            ? Qt.alpha(button.accentColor, 0.18)
            : "transparent"
    }

    Kirigami.Icon {
        anchors.centerIn: parent
        width: button.iconSize
        height: button.iconSize
        source: button.taskModel.Decoration
        selected: button.taskModel.IsActive
    }

    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        spacing: 2
        Repeater {
            model: Math.min(4, button.windowCount)
            Rectangle {
                required property int index
                width: index === 3 && button.windowCount > 4 ? 10 : 5
                height: 3
                radius: 2
                color: button.accentColor
            }
        }
    }

    HoverHandler { id: hover }
    DragHandler { id: dragHandler }

    TapHandler {
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        onTapped: (eventPoint, mouseButton) => {
            if (mouseButton === Qt.MiddleButton)
                button.newInstanceRequested();
            else if (mouseButton === Qt.RightButton)
                button.menuRequested(button.desktopId);
            else
                button.activateRequested();
        }
    }

    DropArea {
        anchors.fill: parent
        onEntered: drag => {
            if (drag.source && drag.source.desktopId && drag.source !== button)
                drag.accepted = true;
        }
        onDropped: drop => {
            if (drop.source && drop.source.desktopId)
                button.moveRequested(drop.source.desktopId, button.desktopId);
        }
    }

    Keys.onReturnPressed: activateRequested()
    Keys.onEnterPressed: activateRequested()
    Keys.onMenuPressed: button.menuRequested(button.desktopId)

    Controls.ToolTip.visible: hover.hovered
    Controls.ToolTip.text: taskModel.Display || desktopId

    Controls.Menu {
        id: menu

        Instantiator {
            model: button.menuActions()
            delegate: Controls.MenuItem {
                required property var modelData
                text: modelData.name
                enabled: modelData.enabled !== false
                onTriggered: {
                    if (modelData.kind === "window")
                        button.tasksSource.requestActivate(button.tasksSource.makeModelIndex(button.taskIndex, modelData.index));
                    else if (modelData.kind === "new")
                        button.newInstanceRequested();
                    else if (modelData.kind === "desktop")
                        button.desktopActionRequested(button.desktopId, modelData.id);
                }
            }
            onObjectAdded: (index, object) => menu.insertItem(index, object)
            onObjectRemoved: (_index, object) => menu.removeItem(object)
        }
        Controls.MenuSeparator {}
        Controls.MenuItem {
            text: button.pinned ? i18n("Unpin from Dock") : i18n("Pin to Dock")
            enabled: button.desktopId.length > 0
            onTriggered: button.pinned
                ? button.unpinRequested(button.desktopId)
                : button.pinRequested(button.desktopId)
        }
        Controls.MenuSeparator {}
        Controls.MenuItem {
            text: i18n("Quit All")
            enabled: button.taskModel.IsClosable === true
            onTriggered: button.closeRequested()
        }
    }

    function menuActions() {
        const actions = [];
        if (windowCount > 1) {
            for (let index = 0; index < windowCount; ++index) {
                const childIndex = tasksSource.makeModelIndex(taskIndex, index);
                actions.push({
                    kind: "window",
                    index: index,
                    name: String(tasksSource.data(childIndex, Qt.DisplayRole) ||
                        i18n("Window %1", index + 1))
                });
            }
        }
        actions.push({
            kind: "new",
            name: i18n("New Window"),
            enabled: taskModel.CanLaunchNewInstance !== false
        });
        for (const action of desktopActions)
            actions.push({kind: "desktop", id: action.id, name: action.name});
        return actions;
    }

    function showContextMenu(actions) {
        desktopActions = actions;
        Qt.callLater(function() { menu.popup(); });
    }

    function normalizedDesktopId(value) {
        let id = String(value);
        if (id.startsWith("applications:"))
            id = id.substring("applications:".length);
        if (id.startsWith("file://"))
            id = id.substring(id.lastIndexOf("/") + 1);
        if (id.length > 0 && !id.endsWith(".desktop"))
            id += ".desktop";
        return id;
    }
}
