interface I { }
interface J { }
struct A : I { }
struct B : J { }
int choose<T : I>(T item) { return 3; }
int choose<T : J>(T item) { return 5; }
int choose(int item) { return item; }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    A a;
    B b;
    output[0] = choose(a);
    output[1] = choose(b);
    output[2] = choose<A>(a);
    output[3] = choose(7);
}
