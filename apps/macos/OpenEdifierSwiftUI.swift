import Foundation
import SwiftUI

struct SpeakerDevice: Codable {
    let id: String
    let name: String
    let model: String
    let host: String
    let addresses: [String]

    var controlHost: String {
        addresses.first(where: { $0.contains(".") }) ?? addresses.first ?? host
    }
}

struct VolumeState: Codable {
    let current: Int
    let min: Int
    let max: Int
}

struct EqualizerState: Codable {
    let preset: Int
    let presetCount: Int

    enum CodingKeys: String, CodingKey {
        case preset
        case presetCount = "preset_count"
    }
}

struct SpeakerCapabilities: Codable {
    let sources: [String]
    let volume: Bool
    let equalizer: Bool
    let playback: Bool
    let events: Bool
}

struct SpeakerStatus: Codable {
    let name: String
    let model: String
    let firmware: String
    let source: String?
    let volume: VolumeState?
    let equalizer: EqualizerState?
    let playback: String?
    let capabilities: SpeakerCapabilities
}

private struct BridgeEnvelope<T: Decodable>: Decodable {
    let ok: Bool
    let data: T?
    let error: String?
}

private enum BridgeError: LocalizedError {
    case message(String)

    var errorDescription: String? {
        switch self {
        case .message(let message): message
        }
    }
}

private enum RustBridge {
    static func discover() throws -> [SpeakerDevice] {
        try call(["command": "discover", "timeout_ms": 2_000])
    }

    static func status(_ device: SpeakerDevice) throws -> SpeakerStatus {
        try call(request("status", device: device))
    }

    static func source(_ source: String, device: SpeakerDevice) throws -> SpeakerStatus {
        var body = request("source", device: device)
        body["source"] = source
        return try call(body)
    }

    static func volume(_ level: Int, device: SpeakerDevice) throws -> SpeakerStatus {
        var body = request("volume", device: device)
        body["level"] = level
        return try call(body)
    }

    static func equalizer(_ preset: Int, device: SpeakerDevice) throws -> SpeakerStatus {
        var body = request("equalizer", device: device)
        body["preset"] = preset
        return try call(body)
    }

    static func playback(_ action: String, device: SpeakerDevice) throws -> SpeakerStatus {
        var body = request("playback", device: device)
        body["action"] = action
        return try call(body)
    }

    private static func request(_ command: String, device: SpeakerDevice) -> [String: Any] {
        ["command": command, "host": device.controlHost, "model": device.model]
    }

    private static func call<T: Decodable>(_ request: [String: Any]) throws -> T {
        let requestData = try JSONSerialization.data(withJSONObject: request)
        guard let requestText = String(data: requestData, encoding: .utf8) else {
            throw BridgeError.message("无法编码控制请求")
        }
        let responsePointer = requestText.withCString(open_edifier_command)
        guard let responsePointer else {
            throw BridgeError.message("Rust 控制层没有返回结果")
        }
        defer { open_edifier_string_free(responsePointer) }

        let responseData = Data(bytes: responsePointer, count: strlen(responsePointer))
        let envelope = try JSONDecoder().decode(BridgeEnvelope<T>.self, from: responseData)
        guard envelope.ok, let data = envelope.data else {
            throw BridgeError.message(envelope.error ?? "未知控制错误")
        }
        return data
    }
}

private struct SourceOption: Identifiable {
    let id: String
    let title: String
    let symbol: String
}

private final class SpeakerStore: ObservableObject {
    private static let sourceMetadata: [String: (title: String, symbol: String)] = [
        "bluetooth": ("蓝牙", "dot.radiowaves.left.and.right"),
        "aux": ("AUX", "cable.connector"),
        "usb": ("USB", "cable.connector.horizontal"),
        "airplay": ("AirPlay", "airplayaudio"),
    ]

    @Published private(set) var devices: [SpeakerDevice] = []
    @Published private(set) var selectedDeviceID = ""
    @Published private(set) var status: SpeakerStatus?
    @Published private(set) var busy = false
    @Published private(set) var connectionText = "正在发现音箱…"
    @Published private(set) var errorMessage: String?
    @Published var volumeLevel = 0.0

    private let selectedDeviceKey = "OpenEdifier.selectedDeviceID"
    private var started = false
    private var operationRevision = 0
    private var refreshTimer: Timer?

    var canControl: Bool { status != nil && !busy }
    var canSetSource: Bool { canControl && !availableSources.isEmpty }
    var canSetVolume: Bool { canControl && status?.capabilities.volume == true }
    var canPlayback: Bool { canControl && status?.capabilities.playback == true }

    var availableSources: [SourceOption] {
        status?.capabilities.sources.map(Self.sourceOption) ?? []
    }

    var volumeRange: ClosedRange<Double> {
        guard let volume = status?.volume else { return 0...100 }
        return Double(volume.min)...Double(volume.max)
    }

    func start() {
        guard !started else { return }
        started = true
        discover()
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 5, repeats: true) { [weak self] _ in
            self?.refreshQuietly()
        }
    }

    deinit {
        refreshTimer?.invalidate()
    }

    func discover() {
        guard !busy else { return }
        beginOperation()
        let revision = operationRevision
        let preferredID = UserDefaults.standard.string(forKey: selectedDeviceKey)
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let devices = try RustBridge.discover()
                guard let device = devices.first(where: { $0.id == preferredID }) ?? devices.first else {
                    DispatchQueue.main.async { [weak self] in
                        guard let self, self.operationRevision == revision else { return }
                        self.devices = []
                        self.selectedDeviceID = ""
                        self.status = nil
                        self.connectionText = "没有发现受支持的音箱"
                        self.busy = false
                    }
                    return
                }
                let status = try RustBridge.status(device)
                DispatchQueue.main.async { [weak self] in
                    guard let self, self.operationRevision == revision else { return }
                    self.devices = devices
                    self.selectedDeviceID = device.id
                    self.apply(status)
                }
            } catch {
                DispatchQueue.main.async { [weak self] in
                    self?.finish(error, revision: revision)
                }
            }
        }
    }

    func selectDevice(_ id: String) {
        guard !busy, let device = devices.first(where: { $0.id == id }) else { return }
        selectedDeviceID = id
        UserDefaults.standard.set(id, forKey: selectedDeviceKey)
        perform { try RustBridge.status(device) }
    }

    func refresh() {
        guard let device = selectedDevice() else { return }
        perform { try RustBridge.status(device) }
    }

    func selectSource(_ source: String) {
        guard let device = selectedDevice() else { return }
        perform { try RustBridge.source(source, device: device) }
    }

    func commitVolume() {
        guard let device = selectedDevice() else { return }
        let level = Int(volumeLevel.rounded())
        perform { try RustBridge.volume(level, device: device) }
    }

    func adjustVolume(_ delta: Int) {
        guard let device = selectedDevice(), let volume = status?.volume else { return }
        let level = min(max(volume.current + delta, volume.min), volume.max)
        perform { try RustBridge.volume(level, device: device) }
    }

    func selectEqualizer(_ preset: Int) {
        guard let device = selectedDevice() else { return }
        perform { try RustBridge.equalizer(preset, device: device) }
    }

    func playback(_ action: String) {
        guard let device = selectedDevice() else { return }
        perform { try RustBridge.playback(action, device: device) }
    }

    private func selectedDevice() -> SpeakerDevice? {
        devices.first(where: { $0.id == selectedDeviceID }) ?? devices.first
    }

    private func beginOperation() {
        operationRevision += 1
        busy = true
        errorMessage = nil
    }

    private func perform(_ operation: @escaping () throws -> SpeakerStatus) {
        guard !busy else { return }
        beginOperation()
        let revision = operationRevision
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let status = try operation()
                DispatchQueue.main.async { [weak self] in
                    guard let self, self.operationRevision == revision else { return }
                    self.apply(status)
                }
            } catch {
                DispatchQueue.main.async { [weak self] in
                    self?.finish(error, revision: revision)
                }
            }
        }
    }

    private func refreshQuietly() {
        guard !busy, let device = selectedDevice() else { return }
        let revision = operationRevision
        let deviceID = device.id
        DispatchQueue.global(qos: .utility).async {
            do {
                let status = try RustBridge.status(device)
                DispatchQueue.main.async { [weak self] in
                    guard
                        let self,
                        !self.busy,
                        self.operationRevision == revision,
                        self.selectedDeviceID == deviceID
                    else { return }
                    self.apply(status)
                }
            } catch {
                DispatchQueue.main.async { [weak self] in
                    guard
                        let self,
                        !self.busy,
                        self.operationRevision == revision,
                        self.selectedDeviceID == deviceID
                    else { return }
                    self.connectionText = "状态同步暂时中断"
                }
            }
        }
    }

    private func apply(_ status: SpeakerStatus) {
        self.status = status
        if let volume = status.volume {
            volumeLevel = Double(volume.current)
        }
        let source = status.source.map(sourceName) ?? "未报告输入源"
        connectionText = "仅通过本地网络连接  ·  \(source)"
        errorMessage = nil
        busy = false
    }

    private func finish(_ error: Error, revision: Int) {
        guard operationRevision == revision else { return }
        errorMessage = error.localizedDescription
        busy = false
    }

    private func sourceName(_ source: String) -> String {
        Self.sourceOption(source).title
    }

    private static func sourceOption(_ source: String) -> SourceOption {
        let metadata = sourceMetadata[source]
        return SourceOption(
            id: source,
            title: metadata?.title ?? source,
            symbol: metadata?.symbol ?? "cable.connector"
        )
    }
}

private struct ContentView: View {
    @ObservedObject var store: SpeakerStore

    var body: some View {
        VStack(spacing: 14) {
            header
            sourceCard
            HStack(spacing: 14) {
                volumeCard
                if store.status?.capabilities.playback == true {
                    playbackCard
                }
            }
            equalizerCard
            footer
            if let error = store.errorMessage {
                Text(error)
                    .font(.callout)
                    .foregroundStyle(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .lineLimit(2)
            }
        }
        .padding(22)
        .frame(minWidth: 620, minHeight: 590)
        .onAppear { store.start() }
    }

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "hifispeaker.2.fill")
                .font(.system(size: 30, weight: .medium))
                .foregroundStyle(.tint)
                .frame(width: 48, height: 48)
            VStack(alignment: .leading, spacing: 3) {
                Text(store.status?.name ?? "OpenEdifier")
                    .font(.title2.weight(.semibold))
                Text(metadata)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if store.devices.count > 1 {
                Picker("音箱", selection: Binding(
                    get: { store.selectedDeviceID },
                    set: store.selectDevice
                )) {
                    ForEach(store.devices, id: \.id) { device in
                        Text(device.name).tag(device.id)
                    }
                }
                .labelsHidden()
                .frame(maxWidth: 180)
                .disabled(store.busy)
            }
            Button(action: store.discover) {
                Image(systemName: "arrow.clockwise")
            }
            .help("重新发现并刷新")
            .disabled(store.busy)
        }
    }

    private var sourceCard: some View {
        GroupBox("输入源") {
            if !store.availableSources.isEmpty, store.status?.source != nil {
                Picker("输入源", selection: Binding(
                    get: { store.status?.source ?? "" },
                    set: store.selectSource
                )) {
                    ForEach(store.availableSources) { source in
                        Label(source.title, systemImage: source.symbol).tag(source.id)
                    }
                }
                .labelsHidden()
                .pickerStyle(.segmented)
                .disabled(!store.canSetSource)
            } else {
                Text("当前设备没有报告可切换的输入源")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private var volumeCard: some View {
        GroupBox("音量") {
            if store.status?.volume != nil {
                VStack(alignment: .leading, spacing: 12) {
                    HStack(alignment: .firstTextBaseline, spacing: 4) {
                        Text(String(Int(store.volumeLevel.rounded())))
                            .font(.system(size: 44, weight: .semibold, design: .monospaced))
                        Text("%")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                        Spacer()
                    }
                    HStack(spacing: 9) {
                        Image(systemName: "speaker.fill")
                            .foregroundStyle(.secondary)
                        Slider(
                            value: $store.volumeLevel,
                            in: store.volumeRange,
                            step: 1,
                            onEditingChanged: { editing in
                                if !editing { store.commitVolume() }
                            }
                        )
                        .accessibilityLabel("音量")
                        .disabled(!store.canSetVolume)
                        Image(systemName: "speaker.wave.3.fill")
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity, minHeight: 112)
            } else {
                Text("当前设备没有报告音量控制")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, minHeight: 112, alignment: .leading)
            }
        }
    }

    private var playbackCard: some View {
        GroupBox {
            VStack(spacing: 18) {
                HStack {
                    Text("播放")
                        .font(.headline)
                    Spacer()
                    Text(playbackName)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                HStack(spacing: 12) {
                    Button { store.playback("previous") } label: {
                        Image(systemName: "backward.end.fill")
                    }
                    Button { store.playback(isPlaying ? "pause" : "play") } label: {
                        Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                    }
                    .controlSize(.large)
                    Button { store.playback("next") } label: {
                        Image(systemName: "forward.end.fill")
                    }
                }
                .buttonStyle(.bordered)
                .disabled(!store.canPlayback)
            }
            .frame(maxWidth: .infinity, minHeight: 112)
        }
    }

    private var equalizerCard: some View {
        GroupBox("音效模式") {
            if let equalizer = store.status?.equalizer {
                Picker("音效模式", selection: Binding(
                    get: { equalizer.preset },
                    set: store.selectEqualizer
                )) {
                    ForEach(0..<equalizer.presetCount, id: \.self) { preset in
                        Text("EQ \(preset + 1)").tag(preset)
                    }
                }
                .labelsHidden()
                .pickerStyle(.segmented)
                .disabled(!store.canControl)
            } else {
                Text("当前设备没有报告 EQ 预设")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(minHeight: 80)
    }

    private var footer: some View {
        HStack(spacing: 7) {
            Image(systemName: "lock.shield")
                .foregroundStyle(.secondary)
            Text(store.connectionText)
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            if store.busy {
                ProgressView()
                    .controlSize(.small)
            }
        }
    }

    private var metadata: String {
        guard let status = store.status else { return "本地音箱控制器" }
        return "在线  ·  \(status.model.uppercased())  ·  固件 \(status.firmware)"
    }

    private var isPlaying: Bool {
        store.status?.playback == "playing"
    }

    private var playbackName: String {
        switch store.status?.playback {
        case "playing": "正在播放"
        case "paused": "已暂停"
        case "stopped": "已停止"
        default: "等待媒体"
        }
    }
}

@main
private struct OpenEdifierApplication: App {
    @StateObject private var store = SpeakerStore()

    var body: some Scene {
        WindowGroup("OpenEdifier") {
            ContentView(store: store)
        }
        .defaultSize(width: 620, height: 590)
        .commands {
            CommandMenu("控制") {
                Button("刷新") { store.discover() }
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(store.busy)
                Divider()
                ForEach(store.availableSources) { source in
                    Button(source.title) { store.selectSource(source.id) }
                        .disabled(!store.canSetSource)
                }
                Divider()
                Button("增大音量") { store.adjustVolume(1) }
                    .keyboardShortcut("=", modifiers: .command)
                    .disabled(!store.canSetVolume)
                Button("减小音量") { store.adjustVolume(-1) }
                    .keyboardShortcut("-", modifiers: .command)
                    .disabled(!store.canSetVolume)
            }
        }
    }
}
