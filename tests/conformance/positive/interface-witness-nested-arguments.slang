interface I<Value> { int read(int input); }
struct Owner<T> { struct Nested { T value; }; }
struct Item<T> : I<T>
{
    int read(int input) { return 6; }
    int read(float input) { return 12; }
}
int read<U, T : I<Owner<U>::Nested>>(T item, U unused) { return item.read(1.0); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item<Owner<int>::Nested> item;
    Item<Owner<float>::Nested> other;
    output[0] = read(item, 0);
    output[1] = read(other, 0.0);
}
