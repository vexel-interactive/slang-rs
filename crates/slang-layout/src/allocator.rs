use crate::{MAX_REGISTER_INDEX, RegisterBinding, RegisterClass, ResourceBinding};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationError {
    InvalidRange,
    Exhausted,
}

impl fmt::Display for AllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRange => "resource register range is invalid or too large",
            Self::Exhausted => "no contiguous resource register range is available",
        })
    }
}

impl std::error::Error for AllocationError {}

/// First-fit allocation over merged occupied intervals, without slot expansion.
#[derive(Clone, Debug, Default)]
pub struct RegisterAllocator {
    occupied: BTreeMap<(RegisterClass, u32), BTreeMap<u64, u64>>,
}

impl RegisterAllocator {
    /// Overlapping explicit ranges remain occupied as their union.
    pub fn reserve(&mut self, binding: ResourceBinding) -> Result<(), AllocationError> {
        if binding.count == 0
            || binding.register.class == RegisterClass::LegacyUniform
            || binding.register.space > MAX_REGISTER_INDEX
            || binding.end() > u64::from(MAX_REGISTER_INDEX) + 1
        {
            return Err(AllocationError::InvalidRange);
        }
        let ranges = self
            .occupied
            .entry((binding.register.class, binding.register.space))
            .or_default();
        let mut start = u64::from(binding.register.index);
        let mut end = binding.end();
        if let Some((&prior_start, &prior_end)) = ranges.range(..=start).next_back()
            && prior_end >= start
        {
            start = prior_start;
            end = end.max(prior_end);
            ranges.remove(&prior_start);
        }
        while let Some((&next_start, &next_end)) = ranges.range(start..).next()
            && next_start <= end
        {
            end = end.max(next_end);
            ranges.remove(&next_start);
        }
        ranges.insert(start, end);
        Ok(())
    }

    pub fn allocate(
        &mut self,
        class: RegisterClass,
        count: u32,
        space: u32,
    ) -> Result<ResourceBinding, AllocationError> {
        if count == 0 || space > MAX_REGISTER_INDEX || class == RegisterClass::LegacyUniform {
            return Err(AllocationError::InvalidRange);
        }
        let mut index = 0;
        if let Some(ranges) = self.occupied.get(&(class, space)) {
            for (&start, &end) in ranges {
                if index + u64::from(count) <= start {
                    break;
                }
                index = end;
            }
        }
        if index + u64::from(count) > u64::from(MAX_REGISTER_INDEX) + 1 {
            return Err(AllocationError::Exhausted);
        }
        let binding = ResourceBinding {
            register: RegisterBinding {
                class,
                index: index as u32,
                space,
            },
            count,
            explicit: false,
        };
        self.reserve(binding)?;
        Ok(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RegisterClass::{Sampler, ShaderResource};

    #[test]
    fn legacy_source_registers_cannot_reserve_or_allocate_resources() {
        let mut allocator = RegisterAllocator::default();
        assert_eq!(
            allocator.allocate(RegisterClass::LegacyUniform, 1, 0),
            Err(AllocationError::InvalidRange)
        );
        let binding = ResourceBinding {
            register: RegisterBinding {
                class: RegisterClass::LegacyUniform,
                index: 0,
                space: 0,
            },
            count: 1,
            explicit: true,
        };
        assert_eq!(
            allocator.reserve(binding),
            Err(AllocationError::InvalidRange)
        );
        assert_eq!(
            allocator
                .allocate(RegisterClass::ConstantBuffer, 1, 0)
                .unwrap()
                .register
                .index,
            0
        );
    }

    fn range(index: u32, count: u32) -> ResourceBinding {
        ResourceBinding {
            register: RegisterBinding {
                class: ShaderResource,
                index,
                space: 0,
            },
            count,
            explicit: true,
        }
    }

    #[test]
    fn first_fit_keeps_small_gaps_after_large_allocations() {
        let mut allocator = RegisterAllocator::default();
        allocator.reserve(range(1, 1)).unwrap();
        assert_eq!(
            allocator
                .allocate(ShaderResource, 2, 0)
                .unwrap()
                .register
                .index,
            2
        );
        assert_eq!(
            allocator
                .allocate(ShaderResource, 1, 0)
                .unwrap()
                .register
                .index,
            0
        );
        assert_eq!(
            allocator
                .allocate(ShaderResource, 1, 0)
                .unwrap()
                .register
                .index,
            4
        );
        assert_eq!(
            allocator
                .allocate(ShaderResource, 1, 1)
                .unwrap()
                .register
                .index,
            0
        );
        assert_eq!(allocator.allocate(Sampler, 1, 0).unwrap().register.index, 0);
    }

    #[test]
    fn overlapping_and_adjacent_ranges_reserve_their_union() {
        let mut allocator = RegisterAllocator::default();
        for binding in [
            range(4, 3),
            range(2, 4),
            range(0, 2),
            range(7, 1),
            range(1, 6),
        ] {
            allocator.reserve(binding).unwrap();
        }
        assert_eq!(
            allocator
                .allocate(ShaderResource, 2, 0)
                .unwrap()
                .register
                .index,
            8
        );
    }

    #[test]
    fn enormous_ranges_and_the_last_slot_do_not_wrap_or_expand() {
        let mut allocator = RegisterAllocator::default();
        allocator.reserve(range(0, MAX_REGISTER_INDEX)).unwrap();
        assert_eq!(
            allocator
                .allocate(ShaderResource, 1, 0)
                .unwrap()
                .register
                .index,
            MAX_REGISTER_INDEX
        );
        assert_eq!(
            allocator.allocate(ShaderResource, 1, 0),
            Err(AllocationError::Exhausted)
        );
        assert_eq!(
            allocator.reserve(range(MAX_REGISTER_INDEX, 2)),
            Err(AllocationError::InvalidRange)
        );
        assert_eq!(
            allocator.allocate(Sampler, 0, 0),
            Err(AllocationError::InvalidRange)
        );
        assert_eq!(
            allocator.allocate(Sampler, 1, MAX_REGISTER_INDEX + 1),
            Err(AllocationError::InvalidRange)
        );
    }
}
