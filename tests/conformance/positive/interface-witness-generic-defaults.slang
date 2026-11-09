T zero<T>() { return T(); }
interface I<Value> { int read(Value input = zero<Value>()); }
struct Item<T> : I<T>
{
    int read(T input) { return 4; }
    int read(float input = 9.0) { return 8; }
}
int read<U, T : I<U>>(T item, U unused) { return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item<int> item;
    Item<uint> other;
    output[0] = read(item, 0);
    output[1] = read(other, 0u);
    output[2] = item.read();
}
