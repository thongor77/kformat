import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    required property var controller
    required property var disk

    readonly property var options: JSON.parse(controller.optionsJson)
    property string fsId: ""
    property string tableId: "gpt"
    property string eraseId: "quick"
    property string label: ""
    property bool showAdvanced: false

    readonly property var currentFs: options.filesystems.find(fs => fs.id === fsId)
    readonly property string labelError: fsId === "" ? "" : controller.labelMessage(fsId, label)
    readonly property bool canContinue: currentFs !== undefined && currentFs.available && labelError === ""

    function choosePreset(preset) {
        fsId = preset.fs
        tableId = preset.table
    }

    function indexOfId(list, id) {
        return list.findIndex(item => item.id === id)
    }

    title: controller.tr("OptionsTitle")

    Component.onCompleted: {
        // Drop any plan left over from another disk before this page starts a new one.
        controller.reset()
        const first = options.presets.find(preset => preset.available)
        if (first) choosePreset(first)
    }

    header: Kirigami.InlineMessage {
        visible: controller.planError !== "" && controller.phase !== "ready"
        type: Kirigami.MessageType.Error
        text: controller.planError
    }

    // Open the confirmation screen once the plan has passed every check.
    Connections {
        target: page.controller
        function onPhaseChanged() {
            const isThisDisk = JSON.parse(page.controller.planJson).device === page.disk.device
            if (page.controller.phase === "ready" && isThisDisk
                    && applicationWindow().pageStack.currentItem === page) {
                applicationWindow().pageStack.push(Qt.resolvedUrl("ConfirmPage.qml"), { controller: page.controller })
            }
        }
    }

    footer: QQC2.ToolBar {
        RowLayout {
            anchors.fill: parent
            QQC2.Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                text: page.disk.title + " (" + page.disk.name + ") — " + page.disk.sizeText
            }
            QQC2.Button {
                text: controller.tr("Continue")
                icon.name: "go-next"
                enabled: page.canContinue && controller.phase !== "planning"
                onClicked: controller.preparePlan(
                    page.disk.device, page.fsId, page.tableId, page.label, page.eraseId
                )
            }
        }
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            level: 3
            text: controller.tr("OptionsQuestion")
        }

        Repeater {
            model: page.options.presets

            delegate: QQC2.ItemDelegate {
                id: card
                required property var modelData
                Layout.fillWidth: true
                enabled: modelData.available
                highlighted: page.fsId === modelData.fs && page.tableId === modelData.table
                onClicked: page.choosePreset(modelData)

                contentItem: ColumnLayout {
                    spacing: 0
                    QQC2.Label {
                        Layout.fillWidth: true
                        font.bold: true
                        text: card.modelData.title
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        opacity: 0.7
                        color: card.modelData.available ? Kirigami.Theme.textColor : Kirigami.Theme.negativeTextColor
                        text: card.modelData.available ? card.modelData.description : card.modelData.unavailableReason
                    }
                }
            }
        }

        Kirigami.FormLayout {
            Layout.fillWidth: true

            QQC2.TextField {
                Kirigami.FormData.label: controller.tr("OptionsLabelField")
                placeholderText: controller.tr("OptionsLabelPlaceholder")
                maximumLength: 255
                text: page.label
                onTextEdited: page.label = text
            }
            QQC2.Label {
                visible: page.labelError !== ""
                color: Kirigami.Theme.negativeTextColor
                wrapMode: Text.Wrap
                text: page.labelError
            }

            QQC2.CheckBox {
                text: controller.tr("OptionsAdvanced")
                checked: page.showAdvanced
                onToggled: page.showAdvanced = checked
            }

            QQC2.ComboBox {
                id: fsBox
                visible: page.showAdvanced
                Kirigami.FormData.label: controller.tr("OptionsFsField")
                model: page.options.filesystems
                textRole: "name"
                valueRole: "id"
                currentIndex: page.indexOfId(page.options.filesystems, page.fsId)
                onActivated: page.fsId = currentValue

                delegate: QQC2.ItemDelegate {
                    required property var modelData
                    required property int index
                    width: fsBox.width
                    enabled: modelData.available
                    highlighted: fsBox.highlightedIndex === index
                    text: modelData.name + (modelData.available ? "" : page.controller.tr("OptionsUnavailableSuffix"))
                }
            }

            QQC2.ComboBox {
                id: tableBox
                visible: page.showAdvanced
                Kirigami.FormData.label: controller.tr("OptionsTableField")
                model: page.options.tables
                textRole: "name"
                valueRole: "id"
                currentIndex: page.indexOfId(page.options.tables, page.tableId)
                onActivated: page.tableId = currentValue
            }

            QQC2.ComboBox {
                visible: page.showAdvanced
                Kirigami.FormData.label: controller.tr("OptionsEraseField")
                textRole: "text"
                valueRole: "id"
                model: [
                    { id: "quick", text: controller.tr("OptionsEraseQuick") },
                    { id: "zero", text: controller.tr("OptionsEraseZero") }
                ]
                currentIndex: page.eraseId === "zero" ? 1 : 0
                onActivated: page.eraseId = currentValue
            }

            QQC2.Label {
                visible: page.showAdvanced && page.currentFs !== undefined && !page.currentFs.available
                color: Kirigami.Theme.negativeTextColor
                wrapMode: Text.Wrap
                text: page.currentFs ? page.currentFs.unavailableReason : ""
            }
        }
    }
}
