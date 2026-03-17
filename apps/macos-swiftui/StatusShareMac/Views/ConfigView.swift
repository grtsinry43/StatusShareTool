import SwiftUI

struct ConfigView: View {
    @Bindable var viewModel: AppViewModel

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                configForm
                actionButtons
                outputSection
            }
            .padding(24)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    // MARK: - Form

    private var configForm: some View {
        Form {
            Section("配置文件") {
                LabeledContent("路径") {
                    TextField("", text: $viewModel.configPath)
                        .textFieldStyle(.roundedBorder)
                }
            }

            Section("服务器连接") {
                LabeledContent("Base URL") {
                    TextField("http://127.0.0.1:3000", text: $viewModel.baseURL)
                        .textFieldStyle(.roundedBorder)
                }

                LabeledContent("Token") {
                    SecureField("gt_...", text: $viewModel.token)
                        .textFieldStyle(.roundedBorder)
                }

                LabeledContent("心跳间隔") {
                    Stepper("\(viewModel.heartbeatInterval) 秒", value: $viewModel.heartbeatInterval, in: 5...600, step: 5)
                }

                LabeledContent("User Agent") {
                    Text(viewModel.userAgent)
                        .foregroundStyle(.secondary)
                        .font(.system(.body, design: .monospaced))
                }
            }
        }
        .formStyle(.grouped)
    }

    // MARK: - Actions

    private var actionButtons: some View {
        HStack(spacing: 12) {
            Button("加载配置") {
                viewModel.loadConfig()
            }
            .buttonStyle(.bordered)

            Button("保存配置") {
                viewModel.saveConfig()
            }
            .buttonStyle(.borderedProminent)

            Spacer()

            Button("获取服务器状态") {
                viewModel.fetchStatus()
            }
            .buttonStyle(.bordered)
        }
    }

    // MARK: - Output

    private var outputSection: some View {
        Group {
            if !viewModel.output.isEmpty {
                GroupBox("结果") {
                    Text(viewModel.output)
                        .font(.system(.body, design: .monospaced))
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                        .padding(8)
                }
            }
        }
    }
}
