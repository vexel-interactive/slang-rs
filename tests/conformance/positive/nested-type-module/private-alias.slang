import lib;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Outer.Hidden value = Outer.Hidden(1); }
