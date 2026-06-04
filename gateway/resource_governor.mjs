export class ResourceGovernor {
  #limit
  #running   = new Set()  // instanceIds currently running
  #suspended = []          // FIFO queue: oldest-suspended first

  constructor(limit) {
    this.#limit = limit
  }

  get limit() { return this.#limit }

  setLimit(n) {
    this.#limit = n
    const excess = Math.max(0, this.#running.size - n)
    const toSuspend = []
    if (excess > 0) {
      const arr = [...this.#running]
      // suspend the last-added ones (deterministic)
      for (let i = arr.length - 1; i >= arr.length - excess; i--) {
        toSuspend.push(arr[i])
        this.#running.delete(arr[i])
        this.#suspended.push(arr[i])
      }
    }
    return toSuspend
  }

  canStart() {
    return this.#running.size < this.#limit
  }

  onStart(instanceId) {
    if (!this.canStart()) throw new Error('resource limit reached')
    this.#running.add(instanceId)
  }

  onSuspend(instanceId) {
    this.#running.delete(instanceId)
    if (!this.#suspended.includes(instanceId)) {
      this.#suspended.push(instanceId)
    }
  }

  onDestroy(instanceId) {
    this.#running.delete(instanceId)
    const i = this.#suspended.indexOf(instanceId)
    if (i !== -1) this.#suspended.splice(i, 1)

    // resume oldest queued instance if capacity allows
    if (this.#suspended.length > 0 && this.#running.size < this.#limit) {
      return this.#suspended.shift()
    }
    return null
  }

  suspendQueue() { return [...this.#suspended] }
  running()      { return this.#running.size }
  suspended()    { return this.#suspended.length }
  total()        { return this.#running.size + this.#suspended.length }
}
