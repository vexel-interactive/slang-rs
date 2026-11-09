struct Box<T> { T value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    int a = 1, b = 2, c = 0;
    Box<int> box = {3};
    output[0] = uint(a < b > (c));
    output[1] = uint(a < box.value > (c));
    output[2] = uint(a < b);
    output[3] = uint((a < b) > c);
    output[4] = uint(true < false);
}
