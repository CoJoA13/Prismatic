// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.plasma.workspace.dbus as DBus
import org.kde.taskmanager as TaskManager

import "../code/panelLogic.mjs" as PanelLogic

PlasmoidItem {
    id: root

    readonly property string serviceName: "io.github.CoJoA13.Prismatic.Service"
    readonly property string servicePath: "/io/github/CoJoA13/Prismatic"
    readonly property string serviceInterface: "io.github.CoJoA13.Prismatic.Service1"
    property double revision: 0
    property var config: ({
        schemaVersion: 1,
        seeded: false,
        favorites: [],
        display: {mode: "primary", connectorByAdapter: {}},
        geometry: {edge: "bottom", lengthMode: "fit", customLengthPercent: 60, alignment: "center", iconSize: 48},
        behavior: {visibility: "dodge", revealDelayMs: 160, hideDelayMs: 300, shortcut: "<Super><Alt>d"},
        appearance: {colorScheme: "system", accent: {mode: "system"}, opacityPercent: 88, cornerRadius: 16}
    })
    property bool applyingLaunchers: false
    property int focusedTask: -1
    property var activationCycles: ({})
    readonly property bool ownsPanel: String(Plasmoid.configuration.ownedPanelId) === String(Plasmoid.containment.id)
    readonly property color surfaceBase: config.appearance.colorScheme === "light"
        ? "#f6f5f4"
        : config.appearance.colorScheme === "dark" ? "#202024" : Kirigami.Theme.backgroundColor
    readonly property color surfaceColor: Qt.rgba(
        surfaceBase.r,
        surfaceBase.g,
        surfaceBase.b,
        Number(config.appearance.opacityPercent) / 100
    )
    readonly property color accentColor: config.appearance.accent.mode === "custom"
        ? config.appearance.accent.color
        : Kirigami.Theme.highlightColor
    property var connectorIds: []

    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.globalShortcut: PanelLogic.plasmaShortcut(config.behavior.shortcut || "<Super><Alt>d")
    Plasmoid.onActivated: {
        Plasmoid.status = PlasmaCore.Types.AcceptingInputStatus;
        root.fullRepresentationItem?.forceActiveFocus();
    }
    Component.onCompleted: {
        connectorIds = screenConnectorIds();
        loadSnapshot();
        registerAdapter(true, ownsPanel ? null : "Applet is not in a Prismatic-owned panel; panel management is disabled");
    }
    Component.onDestruction: registerAdapter(false, "applet destroyed");

    DBus.SignalWatcher {
        service: root.serviceName
        path: root.servicePath
        iface: root.serviceInterface

        function onReceivedSignal(message) {
            if (message.member === "SnapshotChanged" && message.arguments.length === 2)
                root.applySnapshot(message.arguments[0], message.arguments[1]);
        }
    }

    DBus.DBusServiceWatcher {
        busType: DBus.BusType.Session
        watchedService: root.serviceName
        onRegisteredChanged: {
            if (registered) {
                root.loadSnapshot();
                root.registerAdapter(true, root.ownsPanel ? null : "Applet is not in a Prismatic-owned panel; panel management is disabled");
            }
        }
    }

    Connections {
        target: Application

        function onScreensChanged() {
            Qt.callLater(root.refreshScreenTopology);
        }
    }

    TaskManager.TasksModel {
        id: tasksModel
        groupMode: TaskManager.TasksModel.GroupApplications
        sortMode: TaskManager.TasksModel.SortManual
        separateLaunchers: true
        launchInPlace: true
        filterByVirtualDesktop: false
        filterByActivity: false
        filterByScreen: false
        taskReorderingEnabled: true
        launcherList: PanelLogic.launcherUrls(root.config.favorites || [])
    }

    fullRepresentation: FocusScope {
        id: dock
        implicitWidth: Plasmoid.formFactor === PlasmaCore.Types.Vertical
            ? root.config.geometry.iconSize + Kirigami.Units.largeSpacing * 2
            : taskFlow.implicitWidth
        implicitHeight: Plasmoid.formFactor === PlasmaCore.Types.Vertical
            ? taskFlow.implicitHeight
            : root.config.geometry.iconSize + Kirigami.Units.largeSpacing * 2
        activeFocusOnTab: true
        Accessible.name: i18n("Prismatic Dock")
        Accessible.role: Accessible.ToolBar

        Keys.onPressed: event => {
            const backwards = event.key === Qt.Key_Left || event.key === Qt.Key_Up;
            const forwards = event.key === Qt.Key_Right || event.key === Qt.Key_Down;
            if (!backwards && !forwards && event.key !== Qt.Key_Escape)
                return;
            if (event.key === Qt.Key_Escape) {
                dock.focus = false;
                Plasmoid.status = PlasmaCore.Types.PassiveStatus;
                event.accepted = true;
                return;
            }
            const delta = backwards ? -1 : 1;
            root.focusedTask = (root.focusedTask + delta + tasksModel.count) % Math.max(1, tasksModel.count);
            taskRepeater.itemAt(root.focusedTask)?.forceActiveFocus();
            event.accepted = true;
        }

        Rectangle {
            anchors.fill: parent
            z: -2
            radius: Number(root.config.appearance.cornerRadius)
            color: root.surfaceColor
            border.width: 1
            border.color: Qt.alpha(root.accentColor, 0.22)
        }

        Flickable {
            id: flickable
            anchors.fill: parent
            contentWidth: taskFlow.implicitWidth
            contentHeight: taskFlow.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            flickableDirection: Plasmoid.formFactor === PlasmaCore.Types.Vertical
                ? Flickable.VerticalFlick
                : Flickable.HorizontalFlick

            GridLayout {
                id: taskFlow
                rows: Plasmoid.formFactor === PlasmaCore.Types.Vertical ? tasksModel.count : 1
                columns: Plasmoid.formFactor === PlasmaCore.Types.Vertical ? 1 : tasksModel.count
                rowSpacing: Kirigami.Units.smallSpacing
                columnSpacing: Kirigami.Units.smallSpacing

                Repeater {
                    id: taskRepeater
                    model: tasksModel

                    delegate: TaskButton {
                        id: taskButton
                        required property int index
                        taskIndex: index
                        tasksSource: tasksModel
                        iconSize: root.config.geometry.iconSize
                        accentColor: root.accentColor
                        onActivateRequested: root.activateTask(index, desktopId, windowCount)
                        onNewInstanceRequested: tasksModel.requestNewInstance(tasksModel.makeModelIndex(index))
                        onCloseRequested: tasksModel.requestClose(tasksModel.makeModelIndex(index))
                        onPinRequested: desktopId => root.pinDesktop(desktopId)
                        onUnpinRequested: desktopId => root.unpinDesktop(desktopId)
                        onMoveRequested: (desktopId, beforeId) => root.moveDesktop(desktopId, beforeId)
                        onMenuRequested: desktopId => root.openTaskMenu(taskButton, desktopId)
                        onDesktopActionRequested: (desktopId, actionId) => root.launchDesktopAction(desktopId, actionId)
                    }
                }
            }
        }

        DropArea {
            anchors.fill: parent
            z: -1
            onDropped: drop => {
                for (const url of drop.urls || []) {
                    const value = String(url);
                    const marker = "/applications/";
                    const markerIndex = value.lastIndexOf(marker);
                    if (!value.startsWith("file://") || markerIndex < 0 || !value.endsWith(".desktop"))
                        continue;
                    const desktopId = decodeURIComponent(value.substring(markerIndex + marker.length));
                    if (!desktopId.includes("/") && !desktopId.includes("..")) {
                        root.pinDesktop(desktopId);
                        drop.accepted = true;
                        return;
                    }
                }
                drop.accepted = false;
            }
        }
    }

    function message(member, signature, arguments) {
        return DBus.dbusMessage({
            service: serviceName,
            path: servicePath,
            iface: serviceInterface,
            member: member,
            signature: signature,
            arguments: arguments
        });
    }

    function call(member, signature, arguments, resolve, reject) {
        DBus.SessionBus.asyncCall(
            message(member, signature, arguments),
            resolve || function() {},
            reject || function(error) { console.warn("Prismatic D-Bus " + member + ": " + error.message); }
        );
    }

    function loadSnapshot() {
        call("GetSnapshot", "", [], function(reply) {
            applySnapshot(reply[0], reply[1]);
        });
    }

    function applySnapshot(nextRevision, json) {
        let parsed;
        try {
            parsed = JSON.parse(json);
        } catch (error) {
            console.warn("Prismatic configuration parse: " + error.message);
            return;
        }
        if (parsed.schemaVersion !== 1)
            return;
        revision = Number(nextRevision);
        config = parsed;
        tasksModel.launcherList = PanelLogic.launcherUrls(parsed.favorites || []);
        applyPanelProperties();
        registerAdapter(true, ownsPanel ? null : "Applet is not in a Prismatic-owned panel; panel management is disabled");
    }

    function registerAdapter(active, statusMessage) {
        const connectorMissing = PanelLogic.selectedConnectorMissing(config, connectorIds);
        const status = JSON.stringify({
            active: active,
            version: "6.6+",
            capabilities: ["fit", "expand", "custom-length", "autohide", "dodge", "native-hide-timing", "drag-drop", "desktop-actions", "shortcut", "appearance"],
            outputs: connectorIds,
            message: statusMessage || (connectorMissing
                ? "Selected output is disconnected; using the primary screen"
                : null)
        });
        call(active ? "RegisterAdapter" : "UpdateAdapterStatus", "ss", ["plasma", status]);
    }

    function pinDesktop(desktopId) {
        call("Pin", "tss", [revision, desktopId, ""], function(reply) {
            applySnapshot(reply[0], reply[1]);
        });
    }

    function activateTask(row, desktopId, windowCount) {
        if (windowCount <= 1) {
            tasksModel.requestActivate(tasksModel.makeModelIndex(row));
            return;
        }
        const cycles = Object.assign({}, activationCycles);
        const childRow = Number(cycles[desktopId] || 0) % windowCount;
        cycles[desktopId] = childRow + 1;
        activationCycles = cycles;
        tasksModel.requestActivate(tasksModel.makeModelIndex(row, childRow));
    }

    function unpinDesktop(desktopId) {
        call("Unpin", "ts", [revision, desktopId], function(reply) {
            applySnapshot(reply[0], reply[1]);
        });
    }

    function moveDesktop(desktopId, beforeId) {
        call("Move", "tss", [revision, desktopId, beforeId || ""], function(reply) {
            applySnapshot(reply[0], reply[1]);
        });
    }

    function openTaskMenu(button, desktopId) {
        if (!desktopId) {
            button.showContextMenu([]);
            return;
        }
        call("GetDesktopActions", "s", [desktopId], function(reply) {
            button.showContextMenu(PanelLogic.desktopActions(reply[0] || "[]"));
        }, function(error) {
            console.warn("Prismatic desktop actions: " + error.message);
            button.showContextMenu([]);
        });
    }

    function launchDesktopAction(desktopId, actionId) {
        call("LaunchDesktopAction", "ss", [desktopId, actionId]);
    }

    function refreshScreenTopology() {
        connectorIds = screenConnectorIds();
        applyPanelProperties();
        registerAdapter(true, ownsPanel ? null : "Applet is not in a Prismatic-owned panel; panel management is disabled");
    }

    function applyPanelProperties() {
        if (!ownsPanel) {
            console.warn("Prismatic refused to manage an unowned Plasma panel");
            return;
        }
        const properties = PanelLogic.panelProperties(config);
        const desiredScreen = desiredScreenIndex();
        const panelId = Number(Plasmoid.containment.id);
        if (!Number.isFinite(panelId))
            return;
        const script = [
            "const panel = panelById(" + panelId + ");",
            "if (panel) {",
            "panel.location = " + JSON.stringify(properties.location) + ";",
            "panel.lengthMode = " + JSON.stringify(properties.lengthMode) + ";",
            "panel.alignment = " + JSON.stringify(properties.alignment) + ";",
            "panel.height = " + Number(properties.height) + ";",
            "panel.hiding = " + JSON.stringify(properties.hiding) + ";",
            "if (panel.screen !== " + Number(desiredScreen) + ") {",
            "panel.currentConfigGroup = [];",
            "panel.writeConfig('lastScreen', " + Number(desiredScreen) + ");",
            "panel.reloadConfig();",
            "}",
            "const geometry = screenGeometry(panel.screen);",
            "const available = (panel.location === 'left' || panel.location === 'right') ? geometry.height : geometry.width;",
            "panel.maximumLength = panel.lengthMode === 'fit' ? Math.round(available * 0.9) : available;",
            "if (panel.lengthMode === 'custom') {",
            "panel.length = Math.round(available * " + Number(properties.lengthPercent) + " / 100);",
            "}",
            "}"
        ].join("\n");
        DBus.SessionBus.asyncCall(DBus.dbusMessage({
            service: "org.kde.plasmashell",
            path: "/PlasmaShell",
            iface: "org.kde.PlasmaShell",
            member: "evaluateScript",
            signature: "s",
            arguments: [script]
        }), function() {}, function(error) {
            console.warn("Prismatic panel configuration: " + error.message);
        });
    }

    function screenConnectorIds() {
        const connectors = [];
        for (let index = 0; index < Application.screens.length; ++index)
            connectors.push(String(Application.screens[index].name));
        return connectors;
    }

    function desiredScreenIndex() {
        if (config.display.mode === "selected") {
            const selected = String(config.display.connectorByAdapter.plasma || "");
            const selectedIndex = connectorIds.indexOf(selected);
            if (selectedIndex >= 0)
                return selectedIndex;
        }
        // Qt keeps the primary screen first in QGuiApplication::screens.
        return 0;
    }
}
