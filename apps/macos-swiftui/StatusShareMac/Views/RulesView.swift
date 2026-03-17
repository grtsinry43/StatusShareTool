import SwiftUI

struct RulesView: View {
    @Bindable var viewModel: AppViewModel

    var body: some View {
        HSplitView {
            ruleList
                .frame(minWidth: 240, idealWidth: 280, maxWidth: 320)
            ruleDetail
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    // MARK: - Rule list

    private var ruleList: some View {
        VStack(spacing: 0) {
            defaultPolicySection
                .padding(12)

            Divider()

            List(selection: $viewModel.selectedRuleIndex) {
                ForEach(Array(viewModel.matchingConfig.rules.enumerated()), id: \.offset) { index, rule in
                    ruleRow(rule)
                        .tag(index)
                }
            }
            .listStyle(.inset)

            Divider()

            HStack {
                Button(action: viewModel.addRule) {
                    Image(systemName: "plus")
                }
                Button {
                    if let idx = viewModel.selectedRuleIndex {
                        viewModel.deleteRule(at: idx)
                    }
                } label: {
                    Image(systemName: "minus")
                }
                .disabled(viewModel.selectedRuleIndex == nil)
                Spacer()
            }
            .padding(8)
        }
    }

    private var defaultPolicySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Toggle("默认上报", isOn: Binding(
                get: { viewModel.matchingConfig.defaultReport },
                set: { val in
                    viewModel.matchingConfig = MatchEngineConfig(
                        defaultReport: val,
                        defaultDisplayName: viewModel.matchingConfig.defaultDisplayName,
                        defaultExtend: viewModel.matchingConfig.defaultExtend,
                        rules: viewModel.matchingConfig.rules
                    )
                }
            ))
            .font(.caption)

            TextField("默认显示名称", text: Binding(
                get: { viewModel.matchingConfig.defaultDisplayName },
                set: { val in
                    viewModel.matchingConfig = MatchEngineConfig(
                        defaultReport: viewModel.matchingConfig.defaultReport,
                        defaultDisplayName: val,
                        defaultExtend: viewModel.matchingConfig.defaultExtend,
                        rules: viewModel.matchingConfig.rules
                    )
                }
            ))
            .textFieldStyle(.roundedBorder)
            .font(.caption)

            TextField("默认 Extend", text: Binding(
                get: { viewModel.matchingConfig.defaultExtend },
                set: { val in
                    viewModel.matchingConfig = MatchEngineConfig(
                        defaultReport: viewModel.matchingConfig.defaultReport,
                        defaultDisplayName: viewModel.matchingConfig.defaultDisplayName,
                        defaultExtend: val,
                        rules: viewModel.matchingConfig.rules
                    )
                }
            ))
            .textFieldStyle(.roundedBorder)
            .font(.caption)
        }
    }

    private func ruleRow(_ rule: WindowMatchRule) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Circle()
                    .fill(rule.enabled ? .green : .secondary)
                    .frame(width: 6, height: 6)
                Text(rule.displayName.isEmpty ? rule.id : rule.displayName)
                    .font(.body)
                    .lineLimit(1)
            }
            Text("\(fieldLabel(rule.field)) \(kindLabel(rule.kind)) \"\(rule.pattern)\"")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .padding(.vertical, 2)
    }

    // MARK: - Rule detail

    @ViewBuilder
    private var ruleDetail: some View {
        if let index = viewModel.selectedRuleIndex,
           viewModel.matchingConfig.rules.indices.contains(index) {
            RuleEditorView(rule: viewModel.matchingConfig.rules[index]) { updated in
                viewModel.updateRule(at: index, updated)
            }
            .id(index)
        } else {
            ContentUnavailableView("选择规则", systemImage: "list.bullet.rectangle", description: Text("从左侧选择一条规则进行编辑"))
        }
    }
}

// MARK: - Rule Editor (uses local state to avoid Binding into immutable struct)

struct RuleEditorView: View {
    @State private var rule: WindowMatchRule
    let onUpdate: (WindowMatchRule) -> Void

    init(rule: WindowMatchRule, onUpdate: @escaping (WindowMatchRule) -> Void) {
        self._rule = State(initialValue: rule)
        self.onUpdate = onUpdate
    }

    var body: some View {
        ScrollView {
            Form {
                Section("基本信息") {
                    LabeledContent("Rule ID") {
                        Text(rule.id)
                            .font(.system(.body, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .textSelection(.enabled)
                    }

                    Toggle("启用", isOn: $rule.enabled)
                }

                Section("匹配条件") {
                    Picker("匹配字段", selection: $rule.field) {
                        Text("WindowTitle").tag(MatchField.windowTitle)
                        Text("AppName").tag(MatchField.appName)
                        Text("ProcessName").tag(MatchField.processName)
                        Text("ExecutablePath").tag(MatchField.executablePath)
                        Text("BundleId").tag(MatchField.bundleId)
                    }

                    Picker("匹配方式", selection: $rule.kind) {
                        Text("Contains").tag(MatchKind.contains)
                        Text("Exact").tag(MatchKind.exact)
                        Text("Prefix").tag(MatchKind.prefix)
                        Text("Suffix").tag(MatchKind.suffix)
                    }

                    LabeledContent("Pattern") {
                        TextField("匹配模式", text: $rule.pattern)
                            .textFieldStyle(.roundedBorder)
                    }

                    Toggle("区分大小写", isOn: $rule.caseSensitive)
                }

                Section("上报行为") {
                    Picker("策略", selection: $rule.reportPolicy) {
                        Text("Allow").tag(ReportPolicy.allow)
                        Text("Deny").tag(ReportPolicy.deny)
                    }

                    LabeledContent("显示名称") {
                        TextField("显示名称", text: $rule.displayName)
                            .textFieldStyle(.roundedBorder)
                    }

                    LabeledContent("Extend") {
                        TextEditor(text: $rule.extend)
                            .font(.body)
                            .frame(minHeight: 60)
                            .border(Color.secondary.opacity(0.2))
                    }
                }
            }
            .formStyle(.grouped)
            .padding(12)
        }
        .onChange(of: rule.enabled) { _, _ in onUpdate(rule) }
        .onChange(of: rule.field) { _, _ in onUpdate(rule) }
        .onChange(of: rule.kind) { _, _ in onUpdate(rule) }
        .onChange(of: rule.pattern) { _, _ in onUpdate(rule) }
        .onChange(of: rule.caseSensitive) { _, _ in onUpdate(rule) }
        .onChange(of: rule.reportPolicy) { _, _ in onUpdate(rule) }
        .onChange(of: rule.displayName) { _, _ in onUpdate(rule) }
        .onChange(of: rule.extend) { _, _ in onUpdate(rule) }
    }
}

// MARK: - Label helpers

func fieldLabel(_ field: MatchField) -> String {
    switch field {
    case .windowTitle: return "WindowTitle"
    case .appName: return "AppName"
    case .processName: return "ProcessName"
    case .executablePath: return "ExecutablePath"
    case .bundleId: return "BundleId"
    }
}

func kindLabel(_ kind: MatchKind) -> String {
    switch kind {
    case .contains: return "Contains"
    case .exact: return "Exact"
    case .prefix: return "Prefix"
    case .suffix: return "Suffix"
    }
}
