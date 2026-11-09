T copy<T>(T value) { return value; }
T create<T>(T value = copy(T()));
U create<U>(U renamed) { return renamed; }
T relay<T>(T value = create<T>()) { return value; }
T recursive<T>(T value) { return recursive<T>(value); }
int recursive(int value) { return value; }

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(create<int>());
    output[1] = uint(relay<int>());
    output[2] = uint(recursive(3));
}
