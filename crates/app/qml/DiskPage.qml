import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    required property var controller
    property string notice: ""
    readonly property var disks: JSON.parse(controller.disksJson)
    property var selected: null

    title: "Choisir un disque"

    // A refreshed list invalidates the previous selection.
    Connections {
        target: page.controller
        function onDisksJsonChanged() { page.selected = null }
    }

    actions: [
        Kirigami.Action {
            icon.name: "view-refresh"
            text: "Actualiser"
            enabled: !controller.busy
            onTriggered: {
                page.selected = null
                controller.refresh()
            }
        }
    ]

    header: ColumnLayout {
        spacing: 0
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: controller.error !== ""
            type: Kirigami.MessageType.Error
            text: "Impossible de lire les disques : " + controller.error
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.notice !== ""
            type: Kirigami.MessageType.Warning
            text: page.notice
        }
    }

    footer: QQC2.ToolBar {
        RowLayout {
            anchors.fill: parent
            QQC2.Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                text: page.selected ? page.selected.title + " (" + page.selected.name + ")" : "Aucun disque sélectionné"
            }
            QQC2.Button {
                text: "Continuer"
                icon.name: "go-next"
                enabled: page.selected !== null
                onClicked: applicationWindow().pageStack.push(
                    Qt.resolvedUrl("OptionsPage.qml"),
                    { controller: page.controller, disk: page.selected }
                )
            }
        }
    }

    ListView {
        id: list
        model: page.disks
        spacing: Kirigami.Units.smallSpacing

        delegate: QQC2.ItemDelegate {
            id: row
            required property var modelData
            width: ListView.view.width
            enabled: modelData.selectable
            highlighted: page.selected !== null && page.selected.device === modelData.device
            onClicked: page.selected = modelData

            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing
                Kirigami.Icon {
                    source: row.modelData.removable ? "drive-removable-media-usb" : "drive-harddisk"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.large
                    Layout.preferredHeight: Kirigami.Units.iconSizes.large
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        Layout.fillWidth: true
                        font.bold: true
                        elide: Text.ElideRight
                        text: row.modelData.title + " — " + row.modelData.sizeText
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        opacity: 0.7
                        color: row.modelData.selectable ? Kirigami.Theme.textColor : Kirigami.Theme.negativeTextColor
                        text: row.modelData.selectable
                            ? row.modelData.name + " · " + (row.modelData.removable ? "Amovible" : "Interne")
                                + " · " + row.modelData.partitions.length + " partition(s)"
                            : row.modelData.blockedReason
                    }
                }
            }
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0 && !controller.busy && controller.error === ""
            text: "Aucun disque détecté"
            explanation: "Branchez un disque ou une clé USB, puis actualisez."
            icon.name: "drive-removable-media-usb"
        }

        QQC2.BusyIndicator {
            anchors.centerIn: parent
            running: controller.busy
        }
    }
}
