struct Holder<T>
{
    T value;
    T copy() { return T(value); }
    T create() { return T(); }
};
struct Shapes<T>
{
    vector<T, 2> pair;
    matrix<T, 2, 2> square;
    T second() { return pair[1]; }
    vector<T, 2> reverse() { return pair.yx; }
    vector<T, 2> row() { return square[1]; }
    T last() { return square._m11; }
    [mutating] void set(T input) { pair.x = input; }
    vector<T, 2> copy(vector<T, 2> input) { return vector<T, 2>(input); }
};
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Holder<int> holder = {3};
    Shapes<int> shape = {{4, 5}, {{6, 7}, {8, 9}}};
    output[0] = uint(holder.copy());
    output[1] = uint(holder.create());
    output[2] = uint(shape.second());
    output[3] = uint(shape.reverse().y);
    output[4] = uint(shape.row().x);
    output[5] = uint(shape.last());
    shape.set(10);
    output[6] = uint(shape.copy(shape.pair).x);
    Holder<float2> vectorHolder = {{1, 2}};
    output[7] = uint(vectorHolder.create().y);
}
