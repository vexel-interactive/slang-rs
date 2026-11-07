module test;

struct Hidden
{
    float value;
};

public typealias Visible = Hidden;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
