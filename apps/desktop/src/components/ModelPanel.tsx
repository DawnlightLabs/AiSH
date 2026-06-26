export function ModelPanel(props: any) {
  return (
    <div className="panel-card">
      <h2>Models</h2>
      <select value={props.selectedProfileId} onChange={(event) => props.onSelect(event.target.value)}>
        {props.profiles.map((profile: any) => (
          <option key={String(profile.id)} value={String(profile.id)}>
            {String(profile.label ?? profile.id)}
          </option>
        ))}
      </select>
    </div>
  );
}
