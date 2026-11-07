struct First { Second values[2]; };
struct Second { First value; };
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
