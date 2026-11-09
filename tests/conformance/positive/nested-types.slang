RWStructuredBuffer<uint> output;

struct Inner { float unrelated; };
int bias() { return 100; }
static int lookup = 100;
struct Outer
{
    static const int Count = 2;
    static int bias() { return 7; }
    static int lookup() { return 8; }
    typedef int Number;
    typealias Pair = Number[Count];
    typedef Inner Item;
    Inner field;
    struct Inner
    {
        Number value = Count;
        static int shared = 3;
        __init(Number x = Count) { value = x + bias(); }
        [ForceInline] int read(Number x = Count) { return value + x; }
        [mutating] void add(Number x) { value += x; }
        struct Leaf { Number value; };
        typedef Leaf Result;
        Result leaf() { return Result(value); }
        int selected() { return lookup(); }
    };
};
struct Other { struct Inner { int value; }; };
typedef Outer Owner;
Outer::Inner identity(Owner.Inner value) { return value; }
[ForceInline] Outer ignored() { output[30] = 100; return {}; }
static Outer.Inner global = Outer.Inner(1);
struct A_B { struct C { int value; }; };
struct A { struct B_C { int value; }; };
struct _slang_nested_type_5_Outer_Inner { int value; };

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Outer.Inner value = Outer::Inner(3);
    output[0] = uint(value.read());
    value.add(4);
    output[1] = uint(value.value);
    output[2] = uint(value.leaf().value);
    output[3] = uint(global.value);
    Owner.Inner copy = identity(value);
    output[4] = uint(copy.value);
    Outer.Item item = Outer.Item();
    output[5] = uint(item.value);
    Outer.Number number = Outer.Number(5);
    Outer.Pair pair = Outer.Pair(number, 6);
    output[6] = uint(pair[0] + pair[1]);
    Outer.Inner.Leaf leaf = Outer.Inner.Leaf(8);
    output[7] = uint(leaf.value);
    Other.Inner other = Other.Inner(9);
    output[8] = uint(other.value);
    Outer owner;
    owner.Inner local = owner.Inner(10);
    output[9] = uint(local.value);
    owner.Inner array[2] = owner.Inner[2](value, local);
    output[10] = uint(array[1].value);
    Outer.Inner fromIgnored = ignored().Inner(11);
    output[11] = uint(fromIgnored.value);
    output[12] = uint(Outer.Inner.shared);
    Outer.Inner.shared++;
    output[13] = uint(copy.shared);
    A_B.C first = A_B.C(12);
    A.B_C second = A.B_C(13);
    _slang_nested_type_5_Outer_Inner third = _slang_nested_type_5_Outer_Inner(14);
    output[14] = uint(first.value + second.value + third.value);
    output[15] = uint(value.selected());
}
