module math_utils;

__include "math-utils/scale";

public struct Pair
{
    public float left;
    public float right;
};

public float weightedSum(Pair pair)
{
    return scale(pair.left) + pair.right;
}
