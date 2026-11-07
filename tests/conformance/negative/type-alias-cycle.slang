typealias A = B;
typealias B = A;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
