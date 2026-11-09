interface I { int read(); }
struct Item { int read() { return 4; } }
int read<T : I>(T item) { return item.read(); }
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)] void main() { Item item; output[0] = read(item); }
