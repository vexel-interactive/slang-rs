struct Box
{
    int value;
    int read<T>(T ignored) { return value; }
    [mutating] T update<T>(T input) { value++; return input; }
    static T copy<T>(T input) { return input; }
};
static int count = 0;
Box make() { count++; Box box = {2}; return box; }
int index() { count++; return 0; }
Box makeGeneric<T>(T ignored) { Box box = {7}; return box; }
int read<T>(T ignored) { return 0; }

struct Other { T Inner<T>(T input) { return input; } };
struct Outer<T> { struct Inner<U> { U value; }; };

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    int first = make().read<int>(3);
    output[0] = uint(first + count);
    int second = make().copy<int>(4);
    output[1] = uint(second + count);
    Box boxes[2] = {{2}, {3}};
    int third = boxes[index()].update(5);
    output[2] = uint(third + boxes[0].value + count);
    Outer<int>.Inner<int> nested = Outer<int>.Inner<int>(6);
    output[3] = uint(nested.value);
    output[4] = uint(makeGeneric(8).read<int>(9));
}
