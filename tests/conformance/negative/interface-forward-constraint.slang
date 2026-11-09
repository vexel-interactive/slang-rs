interface Read<Value> { Value get(); }
U read<T : Read<U>, U>(T item, U ignored) { return item.get(); }
[shader("compute")] [numthreads(1, 1, 1)] void main() { }
