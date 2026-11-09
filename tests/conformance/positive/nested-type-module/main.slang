import lib;
RWStructuredBuffer<uint> output;
struct Inner { float unrelated; };
int Bias = 100;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Outer.Inner value = Outer.Inner();
    output[0] = uint(value.read());
    Outer.Number number = Outer.Number(7);
    Outer.Inner other = Outer.Inner(number);
    output[1] = uint(other.value);
    output[2] = uint(Outer.Inner.shared);
    Member.Inner member = Member.Inner(4);
    output[3] = uint(member.read());
    output[4] = uint(Member.Inner.count);
}
