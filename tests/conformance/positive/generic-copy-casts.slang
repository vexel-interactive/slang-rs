struct Box<T>
{
    T value;
    T read() { return (T)value; }
};
struct Item { int value; };
struct Shapes<T>
{
    vector<T, 2> pair;
    matrix<T, 2, 2> square;
    vector<T, 2> readPair() { return (vector<T, 2>)pair; }
    matrix<T, 2, 2> readSquare() { return (matrix<T, 2, 2>)square; }
};
struct Outer<T>
{
    struct Inner<U>
    {
        T first;
        U second;
        __init(T a, U b) { first = (T)a; second = (U)b; }
        T read() { return (T)first; }
        U other() { return (U)second; }
        Inner<U> copy() { return this; }
    };
};
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Box<int> scalar = {2};
    Box<int[2]> array = {{3, 4}};
    Box<Item> item = {{5}};
    Box<float2> pair = {{6, 7}};
    Shapes<int> shapes = {{8, 9}, {{10, 11}, {12, 13}}};
    Outer<int>.Inner<float> nested = Outer<int>.Inner<float>(14, 15);
    Outer<float>.Inner<int> other = Outer<float>.Inner<int>(16, 17);
    output[0] = uint(scalar.read());
    output[1] = uint(array.read()[1]);
    output[2] = uint(item.read().value);
    output[3] = uint(pair.read().y);
    output[4] = uint(shapes.readPair().y);
    output[5] = uint(shapes.readSquare()._m11);
    output[6] = uint(nested.copy().read());
    output[7] = uint(nested.other());
    output[8] = uint(other.read());
    output[9] = uint(other.copy().other());
}
