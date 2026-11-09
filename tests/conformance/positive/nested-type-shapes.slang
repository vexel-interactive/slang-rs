RWStructuredBuffer<uint> output;
struct Shape
{
    static const int Count = 2;
    typedef int Component;
    typealias Row = vector<Component, Count>;
    typedef row_major float2x3 Matrix;
    struct Data
    {
        static const int Threads = Count + 1;
        Row row;
        Component values[Count];
        Matrix matrix;
        typedef Row Copy;
    };
};
typedef Shape.Data.Copy Copied;
ConstantBuffer<Shape.Data> settings;
ConstantBuffer<Shape.Matrix> transform;
[shader("compute")]
[numthreads(Shape.Data.Threads, 1, 1)]
void main()
{
    Copied row = Copied(4, 5);
    vector<Shape.Component, Shape.Count> other = {6, 7};
    output[0] = uint(row.x + other.y);
    output[1] = uint(settings.values[1] + settings.row.y);
    output[2] = uint(transform[1][2]);
}
