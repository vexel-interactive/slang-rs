RWStructuredBuffer<uint> output;
static const int Count = 99;
typealias Values = int[Shape::Count];
struct Shape
{
    static const int Next = Shape::Count + 1;
    static const int Count = 3;
    static const bool Enabled = true;
    int data[Next];
    [ForceInline] int read(int values[this.Count]) { return values[Count - 1]; }
    [ForceInline] static int[Count] create() { return {5, 6, 7}; }
};
typealias Alias = Shape;
struct Other { static const int Count = Alias.Next; };
[shader("compute")]
[numthreads(Shape.Count, 1, 1)]
void main()
{
    Values a = {1, 2, 3};
    Shape s = {};
    int b[Alias::Count] = Shape::create();
    int c[Other.Count] = {4, 5, 6, 7};
    vector<int, Alias.Count> vectorValue = {8, 9, 10};
    output[0] = uint(s.read(a));
    output[1] = uint(b[2]);
    output[2] = uint(c[Alias.Count]);
    output[3] = uint(vectorValue.z);
    output[4] = uint(Shape.Enabled);
    output[5] = uint(Shape.Next);
    output[6] = uint(Count);
}
