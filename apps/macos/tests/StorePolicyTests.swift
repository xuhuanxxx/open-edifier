@main
private enum StorePolicyTests {
    static func main() {
        var policy = StorePolicy()
        assert(policy.beginQuietRefresh(busy: false))
        assert(!policy.beginQuietRefresh(busy: false))

        policy.completeQuietRefresh()
        policy.setVolumeEditing(true)
        assert(!policy.beginQuietRefresh(busy: false))

        policy.setVolumeEditing(false)
        assert(!policy.beginQuietRefresh(busy: true))
        assert(policy.beginQuietRefresh(busy: false))
    }
}
