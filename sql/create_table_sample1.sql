create or replace view v_sample1 as
select
    f.name, b.order_id
from
    sch1.foo as f
left join sch2.bar as b
    on f.id = b.id;