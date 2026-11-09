interface I : J { int read(); }
interface J : I { }
struct Item : I { int value; int read() { return value; } }
int read<T : J>(T item) { return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)] void main() { Item item = {5}; output[0] = read(item); }
