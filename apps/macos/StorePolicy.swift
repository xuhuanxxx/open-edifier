struct StorePolicy {
    private(set) var editingVolume = false
    private(set) var quietRefreshPending = false

    mutating func setVolumeEditing(_ editing: Bool) {
        editingVolume = editing
    }

    mutating func beginQuietRefresh(busy: Bool) -> Bool {
        guard !busy, !editingVolume, !quietRefreshPending else { return false }
        quietRefreshPending = true
        return true
    }

    mutating func completeQuietRefresh() {
        quietRefreshPending = false
    }
}
