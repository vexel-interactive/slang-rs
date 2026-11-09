interface I { }
struct Box<T : I> { T value; }
Box<int> item;
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
