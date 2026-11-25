Use ASD-STE100 Simplified Technical English with the user.
Read README.md before changing repository ownership or architecture.
Keep implemented and tested behavior separate from product targets.

Repository scope: Own CAR reading, CAR-specific indexes, the direct CAR getBlock Worker and the fixed sample importer. Host orchestration, bulk publication, scheduling and legacy archive administration belong in Hivezilla. Reuse public Blockzilla formats and readers; do not copy the public CLI implementations.
